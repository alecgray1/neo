mod ops;
mod runtime;

use anyhow::Result;
use std::collections::HashMap;
use std::env;
use std::io::{self, Write};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use tokio::io::{AsyncReadExt, BufReader};
use tokio::sync::{mpsc, oneshot};
use tracing::{error, info, warn};

/// Message types for IPC protocol
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum MessageType {
    // Parent → Plugin
    Start = 1,
    Stop = 2,
    Event = 3,
    Tick = 4,

    // Plugin → Parent (requests)
    Ready = 10,
    Log = 11,
    Emit = 12,
    Error = 13,
    PointReadRequest = 20,
    PointWriteRequest = 21,

    // Parent → Plugin (responses)
    PointReadResponse = 30,
    PointWriteResponse = 31,
}

/// Global state for pending point requests
static NEXT_REQUEST_ID: AtomicU32 = AtomicU32::new(1);
static PENDING_REQUESTS: Mutex<Option<HashMap<u32, oneshot::Sender<Vec<u8>>>>> = Mutex::new(None);

/// Initialize the pending requests map
pub fn init_pending_requests() {
    *PENDING_REQUESTS.lock().unwrap() = Some(HashMap::new());
}

/// Register a pending request and return its ID and receiver
pub fn register_request() -> (u32, oneshot::Receiver<Vec<u8>>) {
    let id = NEXT_REQUEST_ID.fetch_add(1, Ordering::SeqCst);
    let (tx, rx) = oneshot::channel();
    if let Some(ref mut map) = *PENDING_REQUESTS.lock().unwrap() {
        map.insert(id, tx);
    }
    (id, rx)
}

/// Complete a pending request with a response
pub fn complete_request(id: u32, payload: Vec<u8>) {
    if let Some(ref mut map) = *PENDING_REQUESTS.lock().unwrap() {
        if let Some(tx) = map.remove(&id) {
            let _ = tx.send(payload);
        }
    }
}

impl TryFrom<u8> for MessageType {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            1 => Ok(MessageType::Start),
            2 => Ok(MessageType::Stop),
            3 => Ok(MessageType::Event),
            4 => Ok(MessageType::Tick),
            10 => Ok(MessageType::Ready),
            11 => Ok(MessageType::Log),
            12 => Ok(MessageType::Emit),
            13 => Ok(MessageType::Error),
            20 => Ok(MessageType::PointReadRequest),
            21 => Ok(MessageType::PointWriteRequest),
            30 => Ok(MessageType::PointReadResponse),
            31 => Ok(MessageType::PointWriteResponse),
            _ => anyhow::bail!("Unknown message type: {}", value),
        }
    }
}

/// Async message reader using tokio
async fn read_message_async(
    stdin: &mut BufReader<tokio::io::Stdin>,
) -> Result<Option<(MessageType, Vec<u8>)>> {
    // Read length (4 bytes, big-endian)
    let mut len_buf = [0u8; 4];
    match stdin.read_exact(&mut len_buf).await {
        Ok(_) => {}
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e.into()),
    }
    let len = u32::from_be_bytes(len_buf) as usize;

    if len == 0 {
        anyhow::bail!("Empty message");
    }

    // Read message type (1 byte)
    let mut type_buf = [0u8; 1];
    stdin.read_exact(&mut type_buf).await?;
    let msg_type = MessageType::try_from(type_buf[0])?;

    // Read payload
    let payload_len = len - 1;
    let mut payload = vec![0u8; payload_len];
    if payload_len > 0 {
        stdin.read_exact(&mut payload).await?;
    }

    Ok(Some((msg_type, payload)))
}

/// Lifecycle message types that need to be processed by the main loop
#[derive(Debug)]
pub enum LifecycleMessage {
    Start(Vec<u8>),
    Stop,
    Event(Vec<u8>),
    Tick,
    Closed,
}

/// Spawn a background task that reads messages from stdin
/// and dispatches them appropriately
fn spawn_stdin_reader() -> mpsc::Receiver<LifecycleMessage> {
    let (tx, rx) = mpsc::channel::<LifecycleMessage>(32);

    tokio::spawn(async move {
        let mut stdin = BufReader::new(tokio::io::stdin());

        loop {
            match read_message_async(&mut stdin).await {
                Ok(Some((msg_type, payload))) => {
                    match msg_type {
                        MessageType::Start => {
                            let _ = tx.send(LifecycleMessage::Start(payload)).await;
                        }
                        MessageType::Stop => {
                            let _ = tx.send(LifecycleMessage::Stop).await;
                            break;
                        }
                        MessageType::Event => {
                            let _ = tx.send(LifecycleMessage::Event(payload)).await;
                        }
                        MessageType::Tick => {
                            let _ = tx.send(LifecycleMessage::Tick).await;
                        }
                        MessageType::PointReadResponse | MessageType::PointWriteResponse => {
                            // Parse request ID and dispatch to waiting op
                            if let Ok(response) =
                                serde_json::from_slice::<serde_json::Value>(&payload)
                            {
                                if let Some(id) = response.get("id").and_then(|v| v.as_u64()) {
                                    complete_request(id as u32, payload);
                                }
                            }
                        }
                        _ => {
                            warn!("Unexpected message type from parent: {:?}", msg_type);
                        }
                    }
                }
                Ok(None) => {
                    // EOF
                    let _ = tx.send(LifecycleMessage::Closed).await;
                    break;
                }
                Err(e) => {
                    error!("Error reading message: {}", e);
                    let _ = tx.send(LifecycleMessage::Closed).await;
                    break;
                }
            }
        }
    });

    rx
}

/// Write a message to stdout
/// Format: [length: 4 bytes BE][msg_type: 1 byte][payload: length-1 bytes]
fn write_message(msg_type: MessageType, payload: &[u8]) -> Result<()> {
    let mut stdout = io::stdout().lock();

    let len = (1 + payload.len()) as u32;
    stdout.write_all(&len.to_be_bytes())?;
    stdout.write_all(&[msg_type as u8])?;
    stdout.write_all(payload)?;
    stdout.flush()?;

    Ok(())
}

fn main() -> Result<()> {
    // Initialize tracing to stderr (stdout is for IPC)
    tracing_subscriber::fmt()
        .with_writer(io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        error!("Usage: neo-plugin-host <plugin.js>");
        std::process::exit(1);
    }

    let plugin_path = &args[1];
    info!("Loading plugin: {}", plugin_path);

    // Initialize pending requests map
    init_pending_requests();

    // Create tokio runtime
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    rt.block_on(async {
        // Create the JS runtime
        let mut js_runtime = runtime::create_runtime()?;

        // Load and execute the plugin
        let plugin_code = std::fs::read_to_string(plugin_path)?;
        js_runtime
            .execute_script("<plugin>", plugin_code)
            .map_err(|e| anyhow::anyhow!("Failed to execute plugin: {}", e))?;

        // Signal ready
        write_message(MessageType::Ready, &[])?;

        // Spawn background stdin reader
        let mut rx = spawn_stdin_reader();

        // Main message loop
        loop {
            match rx.recv().await {
                Some(LifecycleMessage::Start(payload)) => {
                    info!("Received Start");
                    // Parse config and set Neo.config before calling onStart
                    if let Ok(start_data) = serde_json::from_slice::<serde_json::Value>(&payload) {
                        if let Some(config) = start_data.get("config") {
                            let config_json =
                                serde_json::to_string(config).unwrap_or("{}".to_string());
                            let set_config = format!("globalThis.Neo.config = {};", config_json);
                            let _ = js_runtime.execute_script("<set-config>", set_config);
                        }
                    }
                    runtime::call_lifecycle(&mut js_runtime, "onStart", &payload).await?;
                }
                Some(LifecycleMessage::Stop) => {
                    info!("Received Stop");
                    runtime::call_lifecycle(&mut js_runtime, "onStop", &[]).await?;
                    break;
                }
                Some(LifecycleMessage::Event(payload)) => {
                    runtime::call_lifecycle(&mut js_runtime, "onEvent", &payload).await?;
                }
                Some(LifecycleMessage::Tick) => {
                    runtime::call_lifecycle(&mut js_runtime, "onTick", &[]).await?;
                }
                Some(LifecycleMessage::Closed) | None => {
                    // EOF - parent closed stdin
                    info!("Parent closed connection");
                    break;
                }
            }
        }

        Ok::<_, anyhow::Error>(())
    })?;

    Ok(())
}
