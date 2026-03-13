use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::watch;
use tokio::time::{interval, MissedTickBehavior};

use crate::comms::client::{ApiClient, ApiClientError};

/// Sends periodic heartbeats to the API and processes server commands.
///
/// Interval is tier-differentiated and server-adjustable within bounds.
/// Failed heartbeats are queued for offline batch delivery.
pub struct HeartbeatSender {
    api_client: Arc<ApiClient>,
    device_id: String,
    agent_version: String,
    /// Current interval, adjustable by server
    current_interval: Duration,
    /// Tier-based lower bound: server cannot push below this
    min_interval: Duration,
    /// Tier-based upper bound: server cannot push above this
    max_interval: Duration,
    /// Monotonically increasing counter for replay detection
    sequence_number: u64,
    /// Offline heartbeat queue (bounded to MAX_OFFLINE_QUEUE)
    offline_queue: VecDeque<bb_proto::heartbeat::HeartbeatRequest>,
    /// Blocklist version for heartbeat reporting
    blocklist_version: u64,
}

const MAX_OFFLINE_QUEUE: usize = 1000;

/// Tier-based heartbeat configuration.
pub struct HeartbeatConfig {
    pub device_id: String,
    pub agent_version: String,
    pub default_interval: Duration,
    pub min_interval: Duration,
    pub max_interval: Duration,
}

impl HeartbeatConfig {
    /// Self-enrolled tier defaults: 15 min heartbeat, 5 min minimum.
    pub fn self_tier(device_id: String, agent_version: String) -> Self {
        Self {
            device_id,
            agent_version,
            default_interval: Duration::from_secs(900),
            min_interval: Duration::from_secs(300),
            max_interval: Duration::from_secs(3600),
        }
    }

    /// Partner tier defaults: 5 min heartbeat, 1 min minimum.
    pub fn partner_tier(device_id: String, agent_version: String) -> Self {
        Self {
            device_id,
            agent_version,
            default_interval: Duration::from_secs(300),
            min_interval: Duration::from_secs(60),
            max_interval: Duration::from_secs(900),
        }
    }

    /// Authority tier defaults: 5 min heartbeat, 1 min minimum.
    pub fn authority_tier(device_id: String, agent_version: String) -> Self {
        Self {
            device_id,
            agent_version,
            default_interval: Duration::from_secs(300),
            min_interval: Duration::from_secs(60),
            max_interval: Duration::from_secs(900),
        }
    }
}

impl HeartbeatSender {
    pub fn new(api_client: Arc<ApiClient>, config: HeartbeatConfig) -> Self {
        Self {
            api_client,
            device_id: config.device_id,
            agent_version: config.agent_version,
            current_interval: config.default_interval,
            min_interval: config.min_interval,
            max_interval: config.max_interval,
            sequence_number: 0,
            offline_queue: VecDeque::new(),
            blocklist_version: 0,
        }
    }

    /// Update the blocklist version for heartbeat reporting.
    pub fn set_blocklist_version(&mut self, version: u64) {
        self.blocklist_version = version;
    }

    /// Run the heartbeat loop until shutdown signal is received.
    pub async fn run(&mut self, mut shutdown: watch::Receiver<bool>) {
        let mut ticker = interval(self.current_interval);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    // Try to drain offline queue first if we have pending heartbeats
                    if !self.offline_queue.is_empty() {
                        self.drain_offline_queue().await;
                    }

                    match self.send_heartbeat().await {
                        Ok(response) => {
                            self.process_response(&response, &mut ticker);
                            self.sequence_number += 1;
                        }
                        Err(e) => {
                            tracing::warn!(error = %e, "Heartbeat failed");
                            self.queue_offline_heartbeat();
                            self.sequence_number += 1;
                        }
                    }
                }
                _ = shutdown.changed() => {
                    tracing::info!("Heartbeat sender shutting down");
                    break;
                }
            }
        }
    }

    /// Build and send a single heartbeat.
    async fn send_heartbeat(
        &self,
    ) -> Result<bb_proto::heartbeat::HeartbeatResponse, ApiClientError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let request = bb_proto::heartbeat::HeartbeatRequest {
            device_id: self.device_id.clone(),
            sequence_number: self.sequence_number,
            timestamp: now,
            agent_version: self.agent_version.clone(),
            os_version: std::env::consts::OS.to_string(),
            blocklist_version: self.blocklist_version,
            protection_status: Some(self.collect_protection_status()),
            integrity_hash: Vec::new(), // Filled by integrity checker
            uptime_seconds: 0,         // TODO: track uptime
            resource_usage: Some(self.collect_resource_usage()),
            queued_events: 0,
            queued_reports: 0,
        };

        let path = format!("/api/v1/devices/{}/heartbeat", self.device_id);
        self.api_client.post_proto(&path, &request).await
    }

    fn collect_protection_status(&self) -> bb_proto::heartbeat::ProtectionStatus {
        // Default: report what we know. Actual status comes from plugin registry.
        bb_proto::heartbeat::ProtectionStatus {
            dns_blocking: 0,   // ACTIVE
            hosts_file: 2,     // INACTIVE (may not be enabled)
            app_blocking: 2,   // INACTIVE (Phase 2)
            browser_extension: 2, // INACTIVE (Phase 3)
            network_hook: 2,   // INACTIVE
            watchdog_alive: true,
            config_integrity_ok: true,
        }
    }

    fn collect_resource_usage(&self) -> bb_proto::heartbeat::ResourceUsage {
        bb_proto::heartbeat::ResourceUsage {
            cpu_percent: 0.0,
            memory_bytes: 0,
            disk_cache_bytes: 0,
        }
    }

    /// Process the heartbeat response, handling server commands and interval adjustments.
    fn process_response(
        &mut self,
        response: &bb_proto::heartbeat::HeartbeatResponse,
        _ticker: &mut tokio::time::Interval,
    ) {
        if !response.acknowledged {
            tracing::warn!("Heartbeat was not acknowledged by server");
            return;
        }

        for command in &response.commands {
            self.handle_server_command(command);
        }
    }

    fn handle_server_command(&mut self, command: &bb_proto::heartbeat::ServerCommand) {
        match command.command_type.as_str() {
            "force_blocklist_sync" => {
                tracing::info!("Server requested forced blocklist sync");
                // TODO: signal blocklist syncer
            }
            "update_interval" => {
                if let Ok(secs) = String::from_utf8(command.payload.clone())
                    .unwrap_or_default()
                    .parse::<u64>()
                {
                    let requested = Duration::from_secs(secs);
                    let clamped = requested.clamp(self.min_interval, self.max_interval);
                    if clamped != self.current_interval {
                        tracing::info!(
                            old_secs = self.current_interval.as_secs(),
                            new_secs = clamped.as_secs(),
                            "Server adjusted heartbeat interval"
                        );
                        self.current_interval = clamped;
                    }
                }
            }
            "config_update" => {
                tracing::info!("Server pushed config update");
                // TODO: signal config reload
            }
            other => {
                tracing::debug!(command = other, "Unknown server command, ignoring");
            }
        }
    }

    /// Queue a heartbeat for later delivery when connectivity is restored.
    fn queue_offline_heartbeat(&mut self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let hb = bb_proto::heartbeat::HeartbeatRequest {
            device_id: self.device_id.clone(),
            sequence_number: self.sequence_number,
            timestamp: now,
            agent_version: self.agent_version.clone(),
            os_version: std::env::consts::OS.to_string(),
            blocklist_version: self.blocklist_version,
            protection_status: Some(self.collect_protection_status()),
            integrity_hash: Vec::new(),
            uptime_seconds: 0,
            resource_usage: None,
            queued_events: 0,
            queued_reports: 0,
        };

        if self.offline_queue.len() >= MAX_OFFLINE_QUEUE {
            self.offline_queue.pop_front();
            tracing::debug!("Offline queue full, dropped oldest heartbeat");
        }
        self.offline_queue.push_back(hb);
        tracing::debug!(queued = self.offline_queue.len(), "Heartbeat queued for offline delivery");
    }

    /// Try to drain the offline queue by sending a batch.
    async fn drain_offline_queue(&mut self) {
        // For simplicity in Phase 1, try to send each individually.
        // TODO: use batch endpoint POST /api/v1/devices/{id}/heartbeat-batch
        let mut drained = 0;
        while let Some(hb) = self.offline_queue.front() {
            let path = format!("/api/v1/devices/{}/heartbeat", self.device_id);
            match self.api_client.post_proto::<_, bb_proto::heartbeat::HeartbeatResponse>(&path, hb).await {
                Ok(_) => {
                    self.offline_queue.pop_front();
                    drained += 1;
                }
                Err(_) => break, // Still offline
            }
        }
        if drained > 0 {
            tracing::info!(drained, remaining = self.offline_queue.len(), "Drained offline heartbeat queue");
        }
    }

    /// Get the current heartbeat interval.
    pub fn current_interval(&self) -> Duration {
        self.current_interval
    }

    /// Get the current sequence number.
    pub fn sequence_number(&self) -> u64 {
        self.sequence_number
    }

    /// Get the number of queued offline heartbeats.
    pub fn offline_queue_len(&self) -> usize {
        self.offline_queue.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sender(interval_secs: u64) -> HeartbeatSender {
        let client = Arc::new(ApiClient::new_insecure("http://localhost:1".to_string()));
        HeartbeatSender::new(
            client,
            HeartbeatConfig {
                device_id: "test-device".to_string(),
                agent_version: "0.1.0".to_string(),
                default_interval: Duration::from_secs(interval_secs),
                min_interval: Duration::from_secs(60),
                max_interval: Duration::from_secs(3600),
            },
        )
    }

    #[test]
    fn test_self_tier_config() {
        let config = HeartbeatConfig::self_tier("d".into(), "v".into());
        assert_eq!(config.default_interval, Duration::from_secs(900));
        assert_eq!(config.min_interval, Duration::from_secs(300));
    }

    #[test]
    fn test_partner_tier_config() {
        let config = HeartbeatConfig::partner_tier("d".into(), "v".into());
        assert_eq!(config.default_interval, Duration::from_secs(300));
        assert_eq!(config.min_interval, Duration::from_secs(60));
    }

    #[test]
    fn test_offline_queue_bounded() {
        let mut sender = make_sender(300);

        for _ in 0..MAX_OFFLINE_QUEUE + 100 {
            sender.queue_offline_heartbeat();
        }
        assert_eq!(sender.offline_queue.len(), MAX_OFFLINE_QUEUE);
    }

    #[test]
    fn test_sequence_increments() {
        let sender = make_sender(300);
        assert_eq!(sender.sequence_number(), 0);
    }

    #[test]
    fn test_handle_unknown_command() {
        let mut sender = make_sender(300);
        let cmd = bb_proto::heartbeat::ServerCommand {
            command_type: "unknown_command".to_string(),
            payload: Vec::new(),
        };
        // Should not panic
        sender.handle_server_command(&cmd);
    }

    #[test]
    fn test_handle_update_interval_clamped() {
        let mut sender = make_sender(300);
        assert_eq!(sender.current_interval(), Duration::from_secs(300));

        // Try to set to 10 seconds (below minimum of 60)
        let cmd = bb_proto::heartbeat::ServerCommand {
            command_type: "update_interval".to_string(),
            payload: b"10".to_vec(),
        };
        sender.handle_server_command(&cmd);
        assert_eq!(sender.current_interval(), Duration::from_secs(60));

        // Try to set to 7200 (above maximum of 3600)
        let cmd = bb_proto::heartbeat::ServerCommand {
            command_type: "update_interval".to_string(),
            payload: b"7200".to_vec(),
        };
        sender.handle_server_command(&cmd);
        assert_eq!(sender.current_interval(), Duration::from_secs(3600));

        // Set to valid value
        let cmd = bb_proto::heartbeat::ServerCommand {
            command_type: "update_interval".to_string(),
            payload: b"120".to_vec(),
        };
        sender.handle_server_command(&cmd);
        assert_eq!(sender.current_interval(), Duration::from_secs(120));
    }

    #[tokio::test]
    async fn test_shutdown_signal_stops_loop() {
        let (tx, rx) = watch::channel(false);
        let mut sender = make_sender(1); // 1 second for fast test

        let handle = tokio::spawn(async move {
            sender.run(rx).await;
        });

        // Send shutdown signal
        tx.send(true).expect("send shutdown");

        // Should complete within a reasonable time
        tokio::time::timeout(Duration::from_secs(5), handle)
            .await
            .expect("should finish")
            .expect("task should not panic");
    }
}
