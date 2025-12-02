//! IPC protocol for communicating with plugin host processes

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{ChildStdin, ChildStdout};

/// Message types for IPC protocol
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    // Parent → Plugin
    Start = 1,
    Stop = 2,
    Event = 3,
    Tick = 4,

    // Plugin → Parent
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

/// A message received from or sent to a plugin
#[derive(Debug, Clone)]
pub struct PluginMessage {
    pub msg_type: MessageType,
    pub payload: Vec<u8>,
}

impl PluginMessage {
    pub fn new(msg_type: MessageType, payload: Vec<u8>) -> Self {
        Self { msg_type, payload }
    }

    pub fn empty(msg_type: MessageType) -> Self {
        Self {
            msg_type,
            payload: Vec::new(),
        }
    }

    /// Parse payload as JSON
    pub fn parse_json<T: for<'de> Deserialize<'de>>(&self) -> Result<T> {
        Ok(serde_json::from_slice(&self.payload)?)
    }
}

/// Log message from plugin
#[derive(Debug, Clone, Deserialize)]
pub struct LogMessage {
    pub level: String,
    pub message: String,
}

/// Event emitted by plugin
#[derive(Debug, Clone, Deserialize)]
pub struct EmitMessage {
    #[serde(rename = "type")]
    pub event_type: String,
    pub data: serde_json::Value,
}

/// Point read request from plugin
#[derive(Debug, Clone, Deserialize)]
pub struct PointReadRequest {
    pub id: u32,
    #[serde(rename = "pointId")]
    pub point_id: String,
}

/// Point write request from plugin
#[derive(Debug, Clone, Deserialize)]
pub struct PointWriteRequest {
    pub id: u32,
    #[serde(rename = "pointId")]
    pub point_id: String,
    pub value: serde_json::Value,
}

/// Point read/write response to plugin
#[derive(Debug, Clone, Serialize)]
pub struct PointResponse {
    pub id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// IPC writer for sending messages to a plugin process
pub struct IpcWriter {
    stdin: ChildStdin,
}

impl IpcWriter {
    pub fn new(stdin: ChildStdin) -> Self {
        Self { stdin }
    }

    /// Send a message to the plugin
    pub async fn send(&mut self, msg: &PluginMessage) -> Result<()> {
        let len = (1 + msg.payload.len()) as u32;
        self.stdin.write_all(&len.to_be_bytes()).await?;
        self.stdin.write_all(&[msg.msg_type as u8]).await?;
        self.stdin.write_all(&msg.payload).await?;
        self.stdin.flush().await?;
        Ok(())
    }

    /// Send a message with JSON payload
    pub async fn send_json<T: Serialize>(&mut self, msg_type: MessageType, data: &T) -> Result<()> {
        let payload = serde_json::to_vec(data)?;
        self.send(&PluginMessage::new(msg_type, payload)).await
    }
}

/// IPC reader for receiving messages from a plugin process
pub struct IpcReader {
    stdout: ChildStdout,
}

impl IpcReader {
    pub fn new(stdout: ChildStdout) -> Self {
        Self { stdout }
    }

    /// Receive a message from the plugin
    pub async fn recv(&mut self) -> Result<Option<PluginMessage>> {
        // Read length (4 bytes, big-endian)
        let mut len_buf = [0u8; 4];
        match self.stdout.read_exact(&mut len_buf).await {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => return Err(e.into()),
        }
        let len = u32::from_be_bytes(len_buf) as usize;

        if len == 0 {
            anyhow::bail!("Empty message");
        }

        // Read message type (1 byte)
        let mut type_buf = [0u8; 1];
        self.stdout.read_exact(&mut type_buf).await?;
        let msg_type = MessageType::try_from(type_buf[0])?;

        // Read payload
        let payload_len = len - 1;
        let mut payload = vec![0u8; payload_len];
        if payload_len > 0 {
            self.stdout.read_exact(&mut payload).await?;
        }

        Ok(Some(PluginMessage::new(msg_type, payload)))
    }
}

/// IPC channel for communicating with a plugin process
pub struct IpcChannel {
    stdin: ChildStdin,
    stdout: ChildStdout,
}

impl IpcChannel {
    pub fn new(stdin: ChildStdin, stdout: ChildStdout) -> Self {
        Self { stdin, stdout }
    }

    /// Split into separate reader and writer
    pub fn split(self) -> (IpcWriter, IpcReader) {
        (IpcWriter::new(self.stdin), IpcReader::new(self.stdout))
    }

    /// Send a message to the plugin
    pub async fn send(&mut self, msg: &PluginMessage) -> Result<()> {
        let len = (1 + msg.payload.len()) as u32;
        self.stdin.write_all(&len.to_be_bytes()).await?;
        self.stdin.write_all(&[msg.msg_type as u8]).await?;
        self.stdin.write_all(&msg.payload).await?;
        self.stdin.flush().await?;
        Ok(())
    }

    /// Send a message with JSON payload
    pub async fn send_json<T: Serialize>(&mut self, msg_type: MessageType, data: &T) -> Result<()> {
        let payload = serde_json::to_vec(data)?;
        self.send(&PluginMessage::new(msg_type, payload)).await
    }

    /// Receive a message from the plugin
    pub async fn recv(&mut self) -> Result<Option<PluginMessage>> {
        // Read length (4 bytes, big-endian)
        let mut len_buf = [0u8; 4];
        match self.stdout.read_exact(&mut len_buf).await {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => return Err(e.into()),
        }
        let len = u32::from_be_bytes(len_buf) as usize;

        if len == 0 {
            anyhow::bail!("Empty message");
        }

        // Read message type (1 byte)
        let mut type_buf = [0u8; 1];
        self.stdout.read_exact(&mut type_buf).await?;
        let msg_type = MessageType::try_from(type_buf[0])?;

        // Read payload
        let payload_len = len - 1;
        let mut payload = vec![0u8; payload_len];
        if payload_len > 0 {
            self.stdout.read_exact(&mut payload).await?;
        }

        Ok(Some(PluginMessage::new(msg_type, payload)))
    }
}
