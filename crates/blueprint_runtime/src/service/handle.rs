//! Service Handle
//!
//! A handle to communicate with a running service.

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use tokio::sync::{mpsc, oneshot};

use super::{Event, ServiceError, ServiceResult};

// ─────────────────────────────────────────────────────────────────────────────
// Service State
// ─────────────────────────────────────────────────────────────────────────────

/// Current state of a service
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ServiceState {
    /// Service is starting up
    Starting = 0,
    /// Service is running normally
    Running = 1,
    /// Service is shutting down
    Stopping = 2,
    /// Service has stopped
    Stopped = 3,
    /// Service encountered an error
    Failed = 4,
}

impl ServiceState {
    /// Convert from u8
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Starting,
            1 => Self::Running,
            2 => Self::Stopping,
            3 => Self::Stopped,
            _ => Self::Failed,
        }
    }

    /// Check if the service is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self, ServiceState::Stopped | ServiceState::Failed)
    }

    /// Check if the service is running
    pub fn is_running(&self) -> bool {
        *self == ServiceState::Running
    }
}

impl std::fmt::Display for ServiceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceState::Starting => write!(f, "starting"),
            ServiceState::Running => write!(f, "running"),
            ServiceState::Stopping => write!(f, "stopping"),
            ServiceState::Stopped => write!(f, "stopped"),
            ServiceState::Failed => write!(f, "failed"),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Service Command
// ─────────────────────────────────────────────────────────────────────────────

/// Commands that can be sent to a running service
pub enum ServiceCommand {
    /// Dispatch an event to the service
    Event(Event),

    /// Request graceful shutdown
    Shutdown,

    /// Request current service state
    GetState(oneshot::Sender<ServiceState>),
}

// ─────────────────────────────────────────────────────────────────────────────
// Service Handle
// ─────────────────────────────────────────────────────────────────────────────

/// Handle to communicate with a running service
///
/// This is a lightweight, cloneable handle that can be used to interact with
/// a running service without owning it.
#[derive(Clone)]
pub struct ServiceHandle {
    /// Service identifier
    pub service_id: String,

    /// Command sender channel
    command_tx: mpsc::Sender<ServiceCommand>,

    /// Service state (atomic for lock-free reads)
    state: Arc<AtomicU8>,
}

impl ServiceHandle {
    /// Create a new service handle
    pub(crate) fn new(
        service_id: String,
        command_tx: mpsc::Sender<ServiceCommand>,
        state: Arc<AtomicU8>,
    ) -> Self {
        Self {
            service_id,
            command_tx,
            state,
        }
    }

    /// Get the current service state
    pub fn state(&self) -> ServiceState {
        ServiceState::from_u8(self.state.load(Ordering::SeqCst))
    }

    /// Check if the service is running
    pub fn is_running(&self) -> bool {
        self.state().is_running()
    }

    /// Check if the service is in a terminal state
    pub fn is_terminal(&self) -> bool {
        self.state().is_terminal()
    }

    /// Send an event to this service
    pub async fn send_event(&self, event: Event) -> ServiceResult<()> {
        self.command_tx
            .send(ServiceCommand::Event(event))
            .await
            .map_err(|_| ServiceError::NotRunning(self.service_id.clone()))
    }

    /// Request graceful shutdown
    pub async fn shutdown(&self) -> ServiceResult<()> {
        self.command_tx
            .send(ServiceCommand::Shutdown)
            .await
            .map_err(|_| ServiceError::NotRunning(self.service_id.clone()))
    }

    /// Request state via channel (for when you need to wait for state update)
    pub async fn request_state(&self) -> ServiceResult<ServiceState> {
        let (tx, rx) = oneshot::channel();
        self.command_tx
            .send(ServiceCommand::GetState(tx))
            .await
            .map_err(|_| ServiceError::NotRunning(self.service_id.clone()))?;

        rx.await.map_err(|_| ServiceError::ChannelClosed)
    }
}

impl std::fmt::Debug for ServiceHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServiceHandle")
            .field("service_id", &self.service_id)
            .field("state", &self.state())
            .finish()
    }
}
