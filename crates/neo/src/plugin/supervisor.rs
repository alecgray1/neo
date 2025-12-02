//! Process supervisor with auto-restart and exponential backoff

use std::time::{Duration, Instant};

/// Restart policy configuration
#[derive(Debug, Clone)]
pub struct RestartPolicy {
    /// Maximum number of restarts before giving up
    pub max_restarts: u32,
    /// Initial backoff duration
    pub initial_backoff: Duration,
    /// Maximum backoff duration
    pub max_backoff: Duration,
    /// Backoff multiplier
    pub backoff_multiplier: f64,
    /// Time window for counting restarts (resets after stable period)
    pub restart_window: Duration,
}

impl Default for RestartPolicy {
    fn default() -> Self {
        Self {
            max_restarts: 5,
            initial_backoff: Duration::from_secs(1),
            max_backoff: Duration::from_secs(60),
            backoff_multiplier: 2.0,
            restart_window: Duration::from_secs(300), // 5 minutes
        }
    }
}

/// Supervisor state tracking restarts
#[derive(Debug)]
pub struct Supervisor {
    policy: RestartPolicy,
    restart_count: u32,
    last_restart: Option<Instant>,
    current_backoff: Duration,
    last_stable_time: Option<Instant>,
}

impl Supervisor {
    pub fn new(policy: RestartPolicy) -> Self {
        let initial_backoff = policy.initial_backoff;
        Self {
            policy,
            restart_count: 0,
            last_restart: None,
            current_backoff: initial_backoff,
            last_stable_time: None,
        }
    }

    /// Called when the process starts successfully
    pub fn on_start(&mut self) {
        self.last_stable_time = Some(Instant::now());
    }

    /// Called when the process crashes. Returns the delay before restart,
    /// or None if max restarts exceeded.
    pub fn on_crash(&mut self) -> Option<Duration> {
        let now = Instant::now();

        // Check if we should reset the restart count (stable for restart_window)
        if let Some(stable_time) = self.last_stable_time {
            if now.duration_since(stable_time) >= self.policy.restart_window {
                self.reset();
            }
        }

        self.restart_count += 1;

        if self.restart_count > self.policy.max_restarts {
            return None; // Give up
        }

        // Calculate delay with exponential backoff
        let delay = self.current_backoff;

        // Update backoff for next time
        self.current_backoff = Duration::from_secs_f64(
            (self.current_backoff.as_secs_f64() * self.policy.backoff_multiplier)
                .min(self.policy.max_backoff.as_secs_f64()),
        );

        self.last_restart = Some(now);

        Some(delay)
    }

    /// Reset the supervisor state (called after stable period)
    pub fn reset(&mut self) {
        self.restart_count = 0;
        self.current_backoff = self.policy.initial_backoff;
        self.last_restart = None;
    }

    /// Get the current restart count
    pub fn restart_count(&self) -> u32 {
        self.restart_count
    }

    /// Check if max restarts exceeded
    pub fn is_exhausted(&self) -> bool {
        self.restart_count >= self.policy.max_restarts
    }
}
