//! Project File Watcher
//!
//! Watches project files for changes and broadcasts updates.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use super::loader::ProjectLoader;
use crate::server::{AppState, ChangeType, ServerMessage};

/// File change event
#[derive(Debug, Clone)]
pub enum FileChange {
    /// A device file was modified
    DeviceChanged(String),
    /// A schedule file was modified
    ScheduleChanged(String),
    /// A blueprint file was modified
    BlueprintChanged(String),
    /// The project manifest was modified
    ManifestChanged,
    /// Alarm rules were modified
    AlarmRulesChanged,
}

/// Project file watcher
pub struct ProjectWatcher {
    /// Path to the project
    project_path: PathBuf,
    /// Application state
    state: AppState,
    /// Channel receiver for file events
    rx: mpsc::Receiver<FileChange>,
    /// The underlying watcher (kept alive)
    _watcher: RecommendedWatcher,
}

impl ProjectWatcher {
    /// Create a new project watcher
    pub fn new(project_path: impl AsRef<Path>, state: AppState) -> Result<Self, notify::Error> {
        // Canonicalize the path to get absolute path for reliable comparison
        let project_path = project_path.as_ref().canonicalize()
            .unwrap_or_else(|_| project_path.as_ref().to_path_buf());
        let (tx, rx) = mpsc::channel(100);

        let project_path_clone = project_path.clone();

        // Create the watcher
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            match res {
                Ok(event) => {
                    if let Some(change) = Self::event_to_change(&project_path_clone, &event) {
                        let _ = tx.blocking_send(change);
                    }
                }
                Err(e) => {
                    error!("File watcher error: {}", e);
                }
            }
        })?;

        // Watch the project directory
        watcher.watch(&project_path, RecursiveMode::Recursive)?;
        info!("Watching project directory: {}", project_path.display());

        Ok(Self {
            project_path,
            state,
            rx,
            _watcher: watcher,
        })
    }

    /// Convert a notify event to our FileChange type
    fn event_to_change(project_path: &Path, event: &Event) -> Option<FileChange> {
        info!("File event received: {:?}", event);

        // Only care about modifications and creations
        match event.kind {
            EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_) => {}
            _ => {
                debug!("Ignoring event kind: {:?}", event.kind);
                return None;
            }
        }

        // Get the first path from the event
        let path = event.paths.first()?;

        // Get relative path from project root
        let rel_path = path.strip_prefix(project_path).ok()?;
        let rel_str = rel_path.to_string_lossy();

        info!("File change detected: {}", rel_str);

        // Determine what changed
        if rel_str == "project.toml" {
            Some(FileChange::ManifestChanged)
        } else if rel_str.starts_with("devices/") && rel_str.ends_with(".device.toml") {
            // Extract device ID from filename
            let filename = rel_path.file_stem()?.to_string_lossy();
            let device_id = filename.strip_suffix(".device")?;
            Some(FileChange::DeviceChanged(device_id.to_string()))
        } else if rel_str.starts_with("schedules/") && rel_str.ends_with(".schedule.toml") {
            let filename = rel_path.file_stem()?.to_string_lossy();
            let schedule_id = filename.strip_suffix(".schedule")?;
            Some(FileChange::ScheduleChanged(schedule_id.to_string()))
        } else if rel_str.starts_with("blueprints/") && rel_str.ends_with(".bp.json") {
            let filename = rel_path.file_stem()?.to_string_lossy();
            let blueprint_id = filename.strip_suffix(".bp")?;
            Some(FileChange::BlueprintChanged(blueprint_id.to_string()))
        } else if rel_str == "alarms/rules.toml" {
            Some(FileChange::AlarmRulesChanged)
        } else {
            None
        }
    }

    /// Run the watcher loop
    pub async fn run(mut self) {
        info!("Starting file watcher loop");

        while let Some(change) = self.rx.recv().await {
            self.handle_change(change).await;
        }

        info!("File watcher loop ended");
    }

    /// Handle a file change
    async fn handle_change(&self, change: FileChange) {
        info!("File watcher handling change: {:?}", change);
        match change {
            FileChange::DeviceChanged(device_id) => {
                info!("Device changed: {}", device_id);
                self.reload_device(&device_id).await;
            }
            FileChange::ScheduleChanged(schedule_id) => {
                info!("Schedule changed: {}", schedule_id);
                self.reload_schedule(&schedule_id).await;
            }
            FileChange::BlueprintChanged(blueprint_id) => {
                info!("Blueprint changed: {}", blueprint_id);
                self.reload_blueprint(&blueprint_id).await;
            }
            FileChange::ManifestChanged => {
                info!("Project manifest changed");
                self.reload_project().await;
            }
            FileChange::AlarmRulesChanged => {
                info!("Alarm rules changed");
                // TODO: Reload alarm rules and broadcast
                self.broadcast_change("/alarms/rules", ChangeType::Updated, None)
                    .await;
            }
        }
    }

    /// Reload a specific device
    async fn reload_device(&self, device_id: &str) {
        match ProjectLoader::reload_device(&self.project_path, device_id).await {
            Ok(Some(device)) => {
                // Update in project state
                if let Some(project) = self.state.project().await {
                    // Note: We'd need mutable access to update the project
                    // For now, we just broadcast the change
                    let data = serde_json::to_value(&device).ok();
                    self.broadcast_change(
                        &format!("/devices/{}", device_id),
                        ChangeType::Updated,
                        data,
                    )
                    .await;
                }
            }
            Ok(None) => {
                // Device was deleted
                self.broadcast_change(
                    &format!("/devices/{}", device_id),
                    ChangeType::Deleted,
                    None,
                )
                .await;
            }
            Err(e) => {
                warn!("Failed to reload device {}: {}", device_id, e);
            }
        }
    }

    /// Reload a specific schedule
    async fn reload_schedule(&self, schedule_id: &str) {
        match ProjectLoader::reload_schedule(&self.project_path, schedule_id).await {
            Ok(Some(schedule)) => {
                let data = serde_json::to_value(&schedule).ok();
                self.broadcast_change(
                    &format!("/schedules/{}", schedule_id),
                    ChangeType::Updated,
                    data,
                )
                .await;
            }
            Ok(None) => {
                self.broadcast_change(
                    &format!("/schedules/{}", schedule_id),
                    ChangeType::Deleted,
                    None,
                )
                .await;
            }
            Err(e) => {
                warn!("Failed to reload schedule {}: {}", schedule_id, e);
            }
        }
    }

    /// Reload a specific blueprint
    async fn reload_blueprint(&self, blueprint_id: &str) {
        match ProjectLoader::reload_blueprint(&self.project_path, blueprint_id).await {
            Ok(Some(blueprint)) => {
                // Update in-memory project state
                self.state.update_blueprint(blueprint.clone()).await;

                let data = serde_json::to_value(&blueprint).ok();
                self.broadcast_change(
                    &format!("/blueprints/{}", blueprint_id),
                    ChangeType::Updated,
                    data,
                )
                .await;
            }
            Ok(None) => {
                // Remove from in-memory project state
                self.state.remove_blueprint(blueprint_id).await;

                self.broadcast_change(
                    &format!("/blueprints/{}", blueprint_id),
                    ChangeType::Deleted,
                    None,
                )
                .await;
            }
            Err(e) => {
                warn!("Failed to reload blueprint {}: {}", blueprint_id, e);
            }
        }
    }

    /// Reload the entire project
    async fn reload_project(&self) {
        match ProjectLoader::load(&self.project_path).await {
            Ok(project) => {
                self.state
                    .set_project(project, self.project_path.clone())
                    .await;
                self.broadcast_change("/project", ChangeType::Updated, None)
                    .await;
                info!("Project reloaded successfully");
            }
            Err(e) => {
                error!("Failed to reload project: {}", e);
            }
        }
    }

    /// Broadcast a change to all subscribed clients
    async fn broadcast_change(
        &self,
        path: &str,
        change_type: ChangeType,
        data: Option<serde_json::Value>,
    ) {
        let message = ServerMessage::change(path, change_type, data);
        self.state.broadcast(path, message).await;
    }
}
