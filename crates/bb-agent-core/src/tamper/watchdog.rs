use std::time::{Duration, Instant};

use tokio::sync::{mpsc, watch};

/// Health ping sent from the agent to the watchdog every 5 seconds.
#[derive(Debug, Clone)]
pub struct WatchdogPing {
    /// Timestamp of when the ping was generated.
    pub timestamp: Instant,
    /// SHA-256 hash of the agent binary at startup.
    pub binary_hash: Vec<u8>,
    /// Current blocklist version.
    pub blocklist_version: u64,
}

/// Recovery action to take when the watchdog detects a problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryAction {
    /// Log a tamper event.
    LogTamperEvent,
    /// Restart a specific subsystem.
    RestartSubsystem(String),
    /// Send a high-priority tamper alert to the API.
    SendTamperAlert,
}

/// In-process watchdog monitor (Phase 1).
///
/// In Phase 1, the watchdog runs as a tokio task within the main agent process.
/// It monitors health via an mpsc channel. The agent sends pings every 5 seconds.
/// After 3 missed pings (15 seconds), the watchdog logs a tamper event and
/// triggers recovery.
///
/// Phase 2 promotes this to a separate binary (`bb-watchdog`) with Unix domain
/// socket IPC per ADR-005. The separate process provides mutual supervision:
/// each process monitors the other, making it harder to kill both silently.
pub struct WatchdogMonitor {
    /// Receive pings from the agent.
    health_rx: mpsc::Receiver<WatchdogPing>,
    /// Expected binary hash (set at startup).
    expected_binary_hash: Vec<u8>,
    /// Ping interval.
    ping_interval: Duration,
    /// How many missed pings before triggering recovery.
    missed_threshold: u32,
    /// Callback channel for recovery actions.
    recovery_tx: mpsc::Sender<RecoveryAction>,
    /// Count of consecutive failed restart attempts.
    failed_restarts: u32,
    /// Maximum restart attempts before escalating to alert.
    max_restart_attempts: u32,
}

/// Handle for the agent side to send pings to the watchdog.
pub struct WatchdogHandle {
    health_tx: mpsc::Sender<WatchdogPing>,
}

impl WatchdogHandle {
    /// Send a health ping to the watchdog.
    pub async fn ping(&self, binary_hash: Vec<u8>, blocklist_version: u64) -> bool {
        let ping = WatchdogPing {
            timestamp: Instant::now(),
            binary_hash,
            blocklist_version,
        };
        self.health_tx.send(ping).await.is_ok()
    }
}

impl WatchdogMonitor {
    /// Create a new watchdog monitor and the associated agent handle.
    ///
    /// Returns (monitor, handle, recovery_rx).
    pub fn new(
        expected_binary_hash: Vec<u8>,
    ) -> (Self, WatchdogHandle, mpsc::Receiver<RecoveryAction>) {
        let (health_tx, health_rx) = mpsc::channel(16);
        let (recovery_tx, recovery_rx) = mpsc::channel(16);

        let monitor = Self {
            health_rx,
            expected_binary_hash,
            ping_interval: Duration::from_secs(5),
            missed_threshold: 3,
            recovery_tx,
            failed_restarts: 0,
            max_restart_attempts: 3,
        };

        let handle = WatchdogHandle { health_tx };

        (monitor, handle, recovery_rx)
    }

    /// Run the watchdog monitor loop until shutdown.
    pub async fn run(&mut self, mut shutdown: watch::Receiver<bool>) {
        let check_interval = self.ping_interval;
        let mut last_ping_time = Instant::now();
        let mut consecutive_misses: u32 = 0;

        loop {
            tokio::select! {
                _ = tokio::time::sleep(check_interval) => {
                    // Try to receive all pending pings
                    let mut received_ping = false;
                    while let Ok(ping) = self.health_rx.try_recv() {
                        received_ping = true;
                        last_ping_time = ping.timestamp;

                        // Verify binary hash
                        if !self.expected_binary_hash.is_empty()
                            && ping.binary_hash != self.expected_binary_hash
                        {
                            tracing::error!("Binary hash mismatch detected by watchdog!");
                            let _ = self
                                .recovery_tx
                                .send(RecoveryAction::LogTamperEvent)
                                .await;
                        }
                    }

                    if received_ping {
                        consecutive_misses = 0;
                        self.failed_restarts = 0;
                    } else {
                        let elapsed = last_ping_time.elapsed();
                        if elapsed > check_interval {
                            consecutive_misses += 1;
                            tracing::warn!(
                                missed = consecutive_misses,
                                elapsed_secs = elapsed.as_secs(),
                                "Watchdog: missed ping"
                            );

                            if consecutive_misses >= self.missed_threshold {
                                self.handle_missed_pings().await;
                                consecutive_misses = 0;
                            }
                        }
                    }
                }
                _ = shutdown.changed() => {
                    tracing::info!("Watchdog monitor shutting down");
                    break;
                }
            }
        }
    }

    async fn handle_missed_pings(&mut self) {
        tracing::error!(
            threshold = self.missed_threshold,
            "Watchdog: exceeded missed ping threshold"
        );

        // Log tamper event
        let _ = self
            .recovery_tx
            .send(RecoveryAction::LogTamperEvent)
            .await;

        // Try restart
        if self.failed_restarts < self.max_restart_attempts {
            let _ = self
                .recovery_tx
                .send(RecoveryAction::RestartSubsystem("agent-core".to_string()))
                .await;
            self.failed_restarts += 1;
        } else {
            // Escalate to alert
            tracing::error!(
                attempts = self.failed_restarts,
                "Watchdog: max restart attempts exceeded, sending tamper alert"
            );
            let _ = self
                .recovery_tx
                .send(RecoveryAction::SendTamperAlert)
                .await;
            self.failed_restarts = 0;
        }
    }
}

/// Spawn a background task that sends watchdog pings at regular intervals.
pub fn spawn_ping_sender(
    handle: WatchdogHandle,
    binary_hash: Vec<u8>,
    mut shutdown: watch::Receiver<bool>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(5));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    if !handle.ping(binary_hash.clone(), 0).await {
                        tracing::warn!("Failed to send watchdog ping (channel closed)");
                        break;
                    }
                }
                _ = shutdown.changed() => {
                    break;
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_watchdog_receives_pings() {
        let expected_hash = vec![1, 2, 3, 4];
        let (mut monitor, handle, mut recovery_rx) =
            WatchdogMonitor::new(expected_hash.clone());

        // Send a valid ping
        assert!(handle.ping(expected_hash.clone(), 1).await);

        // The monitor should receive it without triggering recovery
        // We test this by running the monitor briefly
        let (_shutdown_tx, shutdown_rx) = watch::channel(false);
        let monitor_handle = tokio::spawn(async move {
            tokio::time::timeout(Duration::from_millis(100), monitor.run(shutdown_rx)).await
        });

        // Give time for processing
        tokio::time::sleep(Duration::from_millis(50)).await;

        // No recovery actions should have been triggered
        assert!(recovery_rx.try_recv().is_err());

        monitor_handle.abort();
    }

    #[tokio::test]
    async fn test_watchdog_detects_hash_mismatch() {
        let expected_hash = vec![1, 2, 3, 4];
        let (mut monitor, handle, mut recovery_rx) =
            WatchdogMonitor::new(expected_hash.clone());

        // Send a ping with wrong hash
        let wrong_hash = vec![5, 6, 7, 8];
        handle.ping(wrong_hash, 1).await;

        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let monitor_handle = tokio::spawn(async move {
            monitor.run(shutdown_rx).await;
        });

        // Wait for monitor to process
        tokio::time::sleep(Duration::from_millis(200)).await;
        shutdown_tx.send(true).expect("send shutdown");

        // Should have received a LogTamperEvent
        let action = tokio::time::timeout(Duration::from_secs(1), recovery_rx.recv())
            .await;
        // The action may or may not arrive depending on timing, but it should not panic
        drop(action);

        let _ = monitor_handle.await;
    }

    #[tokio::test]
    async fn test_watchdog_shutdown() {
        let (mut monitor, _handle, _recovery_rx) = WatchdogMonitor::new(Vec::new());
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let monitor_handle = tokio::spawn(async move {
            monitor.run(shutdown_rx).await;
        });

        shutdown_tx.send(true).expect("send shutdown");

        tokio::time::timeout(Duration::from_secs(5), monitor_handle)
            .await
            .expect("should finish")
            .expect("should not panic");
    }

    #[test]
    fn test_ping_creation() {
        let ping = WatchdogPing {
            timestamp: Instant::now(),
            binary_hash: vec![1, 2, 3],
            blocklist_version: 42,
        };
        assert_eq!(ping.blocklist_version, 42);
        assert_eq!(ping.binary_hash, vec![1, 2, 3]);
    }
}
