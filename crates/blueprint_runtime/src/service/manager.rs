//! Service Manager
//!
//! Manages the lifecycle of all services, including spawning, stopping, and
//! event routing.

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;

use super::{
    Event, EventPublisher, Service, ServiceCommand, ServiceContext, ServiceError, ServiceHandle,
    ServiceResult, ServiceSpec, ServiceState,
};

// ─────────────────────────────────────────────────────────────────────────────
// Running Service
// ─────────────────────────────────────────────────────────────────────────────

/// Internal representation of a running service
struct RunningService {
    handle: ServiceHandle,
    join_handle: JoinHandle<ServiceResult<()>>,
    spec: ServiceSpec,
}

// ─────────────────────────────────────────────────────────────────────────────
// Service Manager
// ─────────────────────────────────────────────────────────────────────────────

/// Central manager for all services
///
/// The ServiceManager is responsible for:
/// - Spawning new services
/// - Routing events to subscribed services
/// - Managing graceful shutdown
pub struct ServiceManager {
    /// All running services indexed by service_id
    services: DashMap<String, RunningService>,

    /// Broadcast channel for shutdown signal
    shutdown_tx: broadcast::Sender<()>,

    /// Broadcast channel for events
    event_tx: broadcast::Sender<Event>,

    /// Default shutdown timeout
    default_shutdown_timeout: Duration,
}

impl Default for ServiceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ServiceManager {
    /// Create a new service manager
    pub fn new() -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        let (event_tx, _) = broadcast::channel(1024);

        Self {
            services: DashMap::new(),
            shutdown_tx,
            event_tx,
            default_shutdown_timeout: Duration::from_secs(30),
        }
    }

    /// Create a new service manager wrapped in an Arc
    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    /// Get an event publisher for creating events
    pub fn event_publisher(&self) -> EventPublisher {
        EventPublisher::new(self.event_tx.clone())
    }

    /// Spawn a new service
    ///
    /// Returns a handle to the running service.
    pub async fn spawn<S: Service>(&self, service: S) -> ServiceResult<ServiceHandle> {
        self.spawn_with_config(service, serde_json::Value::Null).await
    }

    /// Spawn a new service with configuration
    pub async fn spawn_with_config<S: Service>(
        &self,
        service: S,
        config: serde_json::Value,
    ) -> ServiceResult<ServiceHandle> {
        let spec = service.spec();
        let service_id = spec.id.clone();

        // Check singleton constraint
        if spec.singleton && self.services.contains_key(&service_id) {
            return Err(ServiceError::AlreadyRunning(service_id));
        }

        // Create communication channels
        let (command_tx, command_rx) = mpsc::channel(256);
        let state = Arc::new(AtomicU8::new(ServiceState::Starting as u8));

        let handle = ServiceHandle::new(service_id.clone(), command_tx, Arc::clone(&state));

        // Create service context
        let ctx = ServiceContext::new(
            service_id.clone(),
            config,
            self.event_publisher(),
        );

        // Subscribe to shutdown and events
        let shutdown_rx = self.shutdown_tx.subscribe();
        let event_rx = self.event_tx.subscribe();

        // Spawn the service task
        let tick_interval = spec.tick_interval;
        let subscriptions = spec.subscriptions.clone();
        let shutdown_timeout = spec.shutdown_timeout;
        let state_clone = Arc::clone(&state);

        let join_handle = tokio::spawn(async move {
            run_service_loop(
                service,
                ctx,
                command_rx,
                shutdown_rx,
                event_rx,
                state_clone,
                tick_interval,
                subscriptions,
                shutdown_timeout,
            )
            .await
        });

        // Store the running service
        self.services.insert(
            service_id.clone(),
            RunningService {
                handle: handle.clone(),
                join_handle,
                spec,
            },
        );

        Ok(handle)
    }

    /// Get a service handle by ID
    pub fn get(&self, service_id: &str) -> Option<ServiceHandle> {
        self.services.get(service_id).map(|s| s.handle.clone())
    }

    /// Check if a service is running
    pub fn is_running(&self, service_id: &str) -> bool {
        self.services
            .get(service_id)
            .map(|s| s.handle.is_running())
            .unwrap_or(false)
    }

    /// Publish an event to all subscribed services
    pub fn publish_event(&self, event: Event) {
        let _ = self.event_tx.send(event);
    }

    /// Create and publish an event
    pub fn emit(
        &self,
        event_type: impl Into<String>,
        source: impl Into<String>,
        data: serde_json::Value,
    ) {
        self.publish_event(Event::new(event_type, source, data));
    }

    /// Stop a specific service
    pub async fn stop(&self, service_id: &str) -> ServiceResult<()> {
        let running = self
            .services
            .remove(service_id)
            .map(|(_, v)| v)
            .ok_or_else(|| ServiceError::NotRunning(service_id.to_string()))?;

        // Request shutdown
        let _ = running.handle.shutdown().await;

        // Wait for the task to complete
        let timeout = running.spec.shutdown_timeout;
        match tokio::time::timeout(timeout, running.join_handle).await {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(e)) => {
                tracing::error!(service_id, error = ?e, "Service task panicked");
                Err(ServiceError::Internal(format!("Task panicked: {:?}", e)))
            }
            Err(_) => {
                tracing::warn!(service_id, "Service shutdown timed out");
                Err(ServiceError::ShutdownTimeout)
            }
        }
    }

    /// Initiate graceful shutdown of all services
    pub async fn shutdown_all(&self) -> ServiceResult<()> {
        tracing::info!("Initiating shutdown of all services");

        // Send shutdown signal
        let _ = self.shutdown_tx.send(());

        // Collect all services
        let services: Vec<_> = self
            .services
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().spec.shutdown_timeout))
            .collect();

        // Request shutdown for each service
        for (service_id, _) in &services {
            if let Some(handle) = self.get(service_id) {
                let _ = handle.shutdown().await;
            }
        }

        // Wait for all services to stop
        let timeout = self.default_shutdown_timeout;
        let result = tokio::time::timeout(timeout, async {
            while !self.services.is_empty() {
                // Remove stopped services
                self.services.retain(|_, v| !v.handle.is_terminal());
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await;

        if result.is_err() {
            tracing::warn!("Some services did not stop within timeout");
            return Err(ServiceError::ShutdownTimeout);
        }

        Ok(())
    }

    /// List all running services
    pub fn list(&self) -> Vec<(String, ServiceState)> {
        self.services
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().handle.state()))
            .collect()
    }

    /// Get the number of running services
    pub fn len(&self) -> usize {
        self.services.len()
    }

    /// Check if there are no running services
    pub fn is_empty(&self) -> bool {
        self.services.is_empty()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Service Task Loop
// ─────────────────────────────────────────────────────────────────────────────

/// Run the main service loop
async fn run_service_loop<S: Service>(
    mut service: S,
    ctx: ServiceContext,
    mut command_rx: mpsc::Receiver<ServiceCommand>,
    mut shutdown_rx: broadcast::Receiver<()>,
    mut event_rx: broadcast::Receiver<Event>,
    state: Arc<AtomicU8>,
    tick_interval: Option<Duration>,
    subscriptions: Vec<String>,
    shutdown_timeout: Duration,
) -> ServiceResult<()> {
    // Set state to starting
    state.store(ServiceState::Starting as u8, Ordering::SeqCst);

    // Call on_start
    if let Err(e) = service.on_start(&ctx).await {
        tracing::error!(
            service_id = %ctx.service_id,
            error = %e,
            "Service failed to start"
        );
        state.store(ServiceState::Failed as u8, Ordering::SeqCst);
        return Err(e);
    }

    // Set state to running
    state.store(ServiceState::Running as u8, Ordering::SeqCst);
    tracing::info!(service_id = %ctx.service_id, "Service started");

    // Create tick timer if configured
    let mut tick_timer = tick_interval.map(tokio::time::interval);

    // Main event loop
    loop {
        tokio::select! {
            // Global shutdown signal
            _ = shutdown_rx.recv() => {
                tracing::debug!(service_id = %ctx.service_id, "Received global shutdown signal");
                break;
            }

            // Direct commands
            Some(cmd) = command_rx.recv() => {
                match cmd {
                    ServiceCommand::Shutdown => {
                        tracing::debug!(service_id = %ctx.service_id, "Received shutdown command");
                        break;
                    }
                    ServiceCommand::Event(event) => {
                        if let Err(e) = service.on_event(&ctx, event).await {
                            tracing::warn!(
                                service_id = %ctx.service_id,
                                error = %e,
                                "Error handling direct event"
                            );
                        }
                    }
                    ServiceCommand::GetState(tx) => {
                        let current = ServiceState::from_u8(state.load(Ordering::SeqCst));
                        let _ = tx.send(current);
                    }
                    ServiceCommand::ForceTick => {
                        if let Err(e) = service.on_tick(&ctx).await {
                            tracing::warn!(
                                service_id = %ctx.service_id,
                                error = %e,
                                "Error during forced tick"
                            );
                        }
                    }
                }
            }

            // Broadcast events
            Ok(event) = event_rx.recv() => {
                // Check if event matches any subscription
                if subscriptions.iter().any(|p| event.matches(p)) {
                    if let Err(e) = service.on_event(&ctx, event).await {
                        tracing::warn!(
                            service_id = %ctx.service_id,
                            error = %e,
                            "Error handling broadcast event"
                        );
                    }
                }
            }

            // Tick timer
            _ = async {
                if let Some(ref mut timer) = tick_timer {
                    timer.tick().await
                } else {
                    std::future::pending::<tokio::time::Instant>().await
                }
            } => {
                if let Err(e) = service.on_tick(&ctx).await {
                    tracing::warn!(
                        service_id = %ctx.service_id,
                        error = %e,
                        "Error during tick"
                    );
                }
            }
        }
    }

    // Shutdown phase
    state.store(ServiceState::Stopping as u8, Ordering::SeqCst);
    tracing::debug!(service_id = %ctx.service_id, "Service stopping");

    // Call on_stop with timeout
    let stop_result = tokio::time::timeout(shutdown_timeout, service.on_stop(&ctx)).await;

    match stop_result {
        Ok(Ok(())) => {
            state.store(ServiceState::Stopped as u8, Ordering::SeqCst);
            tracing::info!(service_id = %ctx.service_id, "Service stopped");
            Ok(())
        }
        Ok(Err(e)) => {
            state.store(ServiceState::Failed as u8, Ordering::SeqCst);
            tracing::error!(service_id = %ctx.service_id, error = %e, "Service stop failed");
            Err(e)
        }
        Err(_) => {
            state.store(ServiceState::Failed as u8, Ordering::SeqCst);
            tracing::error!(service_id = %ctx.service_id, "Service stop timed out");
            Err(ServiceError::ShutdownTimeout)
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    struct CountingService {
        id: String,
        start_count: Arc<AtomicUsize>,
        stop_count: Arc<AtomicUsize>,
        event_count: Arc<AtomicUsize>,
        tick_count: Arc<AtomicUsize>,
    }

    impl CountingService {
        fn new(id: &str) -> (Self, Arc<AtomicUsize>, Arc<AtomicUsize>, Arc<AtomicUsize>, Arc<AtomicUsize>) {
            let start_count = Arc::new(AtomicUsize::new(0));
            let stop_count = Arc::new(AtomicUsize::new(0));
            let event_count = Arc::new(AtomicUsize::new(0));
            let tick_count = Arc::new(AtomicUsize::new(0));

            (
                Self {
                    id: id.to_string(),
                    start_count: Arc::clone(&start_count),
                    stop_count: Arc::clone(&stop_count),
                    event_count: Arc::clone(&event_count),
                    tick_count: Arc::clone(&tick_count),
                },
                start_count,
                stop_count,
                event_count,
                tick_count,
            )
        }
    }

    #[async_trait::async_trait]
    impl Service for CountingService {
        fn spec(&self) -> ServiceSpec {
            ServiceSpec::new(&self.id, "Counting Service")
                .subscribe("TestEvent")
        }

        async fn on_start(&mut self, _ctx: &ServiceContext) -> ServiceResult<()> {
            self.start_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn on_stop(&mut self, _ctx: &ServiceContext) -> ServiceResult<()> {
            self.stop_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn on_event(&mut self, _ctx: &ServiceContext, _event: Event) -> ServiceResult<()> {
            self.event_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn on_tick(&mut self, _ctx: &ServiceContext) -> ServiceResult<()> {
            self.tick_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_spawn_and_stop_service() {
        let manager = ServiceManager::new();
        let (service, start_count, stop_count, _, _) = CountingService::new("test-service");

        // Spawn service
        let handle = manager.spawn(service).await.unwrap();

        // Wait a bit for the service to start
        tokio::time::sleep(Duration::from_millis(50)).await;

        assert_eq!(start_count.load(Ordering::SeqCst), 1);
        assert!(handle.is_running());

        // Stop service
        manager.stop("test-service").await.unwrap();

        assert_eq!(stop_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_event_routing() {
        let manager = ServiceManager::new();
        let (service, _, _, event_count, _) = CountingService::new("event-service");

        manager.spawn(service).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Publish a matching event
        manager.emit("TestEvent", "test", serde_json::json!({}));
        tokio::time::sleep(Duration::from_millis(50)).await;

        assert!(event_count.load(Ordering::SeqCst) >= 1);

        manager.shutdown_all().await.unwrap();
    }

    #[tokio::test]
    async fn test_singleton_constraint() {
        let manager = ServiceManager::new();
        let (service1, _, _, _, _) = CountingService::new("singleton-service");
        let (service2, _, _, _, _) = CountingService::new("singleton-service");

        manager.spawn(service1).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;

        let result = manager.spawn(service2).await;
        assert!(matches!(result, Err(ServiceError::AlreadyRunning(_))));

        manager.shutdown_all().await.unwrap();
    }
}
