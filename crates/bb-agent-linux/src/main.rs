mod nftables;
mod platform;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use tokio::sync::watch;

use bb_agent_core::comms::certificate::{CertificateStore, FileCertificateStore};
use bb_agent_core::comms::client::{ApiClient, RetryConfig};
use bb_agent_core::comms::heartbeat::{HeartbeatConfig, HeartbeatSender};
use bb_agent_core::comms::registration::RegistrationService;
use bb_agent_core::comms::reporter::EventReporter;
// BlocklistSyncer is available but started on-demand after registration
#[allow(unused_imports)]
use bb_agent_core::comms::sync::BlocklistSyncer;
use bb_agent_core::config::AgentConfig;
use bb_agent_core::events::emitter::EventEmitter;
use bb_agent_core::events::store::EventStore;
use bb_agent_core::tamper::integrity::BinaryIntegrity;
use bb_agent_core::tamper::watchdog::WatchdogMonitor;
use bb_common::enums::EnrollmentTier;
use bb_common::models::ReportingConfig;

use nftables::NftablesManager;

const AGENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// BetBlocker Agent for Linux -- gambling site blocking service.
#[derive(Parser, Debug)]
#[command(name = "bb-agent-linux", version, about)]
struct Cli {
    /// Path to the configuration directory.
    #[arg(long, default_value = "/var/lib/betblocker")]
    config_dir: PathBuf,

    /// Enrollment token for initial device registration.
    #[arg(long)]
    enroll: Option<String>,

    /// Path to the configuration file.
    #[arg(long)]
    config: Option<PathBuf>,
}

#[tokio::main]
async fn main() {
    // Initialize tracing with journald-compatible format
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    tracing::info!(
        version = AGENT_VERSION,
        config_dir = %cli.config_dir.display(),
        "BetBlocker Agent (Linux) starting"
    );

    if let Err(e) = run(cli).await {
        tracing::error!(error = %e, "Agent failed");
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    // --- Phase 1: Setup ---

    // Create shutdown signal channel
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // Setup signal handling
    #[cfg(unix)]
    {
        let shutdown_tx_clone = shutdown_tx.clone();
        tokio::spawn(async move {
            let mut sigterm =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                    .expect("Failed to register SIGTERM handler");
            let mut sigint =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
                    .expect("Failed to register SIGINT handler");
            let mut sighup =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup())
                    .expect("Failed to register SIGHUP handler");

            tokio::select! {
                _ = sigterm.recv() => {
                    tracing::info!("Received SIGTERM, initiating shutdown");
                    let _ = shutdown_tx_clone.send(true);
                }
                _ = sigint.recv() => {
                    tracing::info!("Received SIGINT, initiating shutdown");
                    let _ = shutdown_tx_clone.send(true);
                }
                _ = sighup.recv() => {
                    tracing::info!("Received SIGHUP, reloading configuration");
                    // TODO: reload config without full restart
                }
            }
        });
    }

    // Non-unix fallback for signal handling (Ctrl+C only)
    #[cfg(not(unix))]
    {
        let shutdown_tx_clone = shutdown_tx.clone();
        tokio::spawn(async move {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to register Ctrl+C handler");
            tracing::info!("Received Ctrl+C, initiating shutdown");
            let _ = shutdown_tx_clone.send(true);
        });
    }

    // Ensure data directories exist
    #[cfg(unix)]
    platform::ensure_directories()
        .map_err(|e| format!("Failed to create directories: {e}"))?;

    // Load configuration
    let config_path = cli
        .config
        .unwrap_or_else(|| cli.config_dir.join("agent.toml"));
    let config = if config_path.exists() {
        AgentConfig::load(&config_path)
            .map_err(|e| format!("Failed to load config: {e}"))?
    } else {
        tracing::info!(
            path = %config_path.display(),
            "Config file not found, using defaults"
        );
        AgentConfig::default()
    };

    // Initialize event store
    let events_db_path = config.data_dir.join("events.db");
    let event_store = EventStore::new(&events_db_path)
        .map_err(|e| format!("Failed to open event store: {e}"))?;
    let event_emitter = EventEmitter::new(event_store);

    // Initialize certificate store
    let cert_store = Arc::new(
        FileCertificateStore::new(&config.data_dir)
            .map_err(|e| format!("Failed to init cert store: {e}"))?,
    );

    // Initialize binary integrity checker
    let binary_integrity = match BinaryIntegrity::new(None) {
        Ok(bi) => {
            tracing::info!(
                hash = bi.startup_hash_hex(),
                "Binary integrity check passed"
            );
            Some(bi)
        }
        Err(e) => {
            tracing::warn!(error = %e, "Binary integrity check skipped");
            None
        }
    };

    // Get binary hash for watchdog
    let binary_hash = binary_integrity
        .as_ref()
        .map(|bi| bi.startup_hash().to_vec())
        .unwrap_or_default();

    // --- Phase 2: Registration ---

    // Build initial API client (without mTLS identity for registration)
    let ca_pem = cert_store
        .load_ca_chain()
        .ok()
        .flatten();

    // Determine if we need to register
    let device_id = if let Some(enrollment_token) = &cli.enroll {
        // New enrollment flow
        tracing::info!("Starting device enrollment");

        // Use a temporary client without mTLS for registration
        // In production this would use the bundled CA cert
        let api_client = Arc::new(ApiClient::new_insecure(config.api_url.clone()));
        let reg_service = RegistrationService::new(api_client.clone(), cert_store.clone());

        let result = reg_service
            .register(enrollment_token, AGENT_VERSION)
            .await
            .map_err(|e| format!("Registration failed: {e}"))?;

        tracing::info!(
            device_id = %result.device_id,
            "Device registered successfully"
        );

        result.device_id
    } else if let Some(id) = &config.device_id {
        tracing::info!(device_id = %id, "Using existing device ID from config");
        id.clone()
    } else {
        return Err("No device ID and no enrollment token. Run with --enroll <token>".into());
    };

    // Build the authenticated API client with mTLS
    let api_client = if let Some(identity_pem) = cert_store.load_identity().ok().flatten() {
        if let Some(ca_chain) = &ca_pem {
            match ApiClient::new(
                config.api_url.clone(),
                ca_chain,
                Some(&identity_pem),
                RetryConfig::default(),
            ) {
                Ok(client) => {
                    tracing::info!("mTLS API client initialized");
                    Arc::new(client)
                }
                Err(e) => {
                    tracing::warn!(error = %e, "mTLS init failed, using unauthenticated client");
                    Arc::new(ApiClient::new_insecure(config.api_url.clone()))
                }
            }
        } else {
            tracing::warn!("No CA chain available, using unauthenticated client");
            Arc::new(ApiClient::new_insecure(config.api_url.clone()))
        }
    } else {
        tracing::warn!("No device identity available, using unauthenticated client");
        Arc::new(ApiClient::new_insecure(config.api_url.clone()))
    };

    api_client.set_device_id(device_id.clone()).await;

    // --- Phase 3: Initialize subsystems ---

    // Initialize plugin registry
    let mut plugin_registry = bb_agent_plugins::PluginRegistry::with_defaults();
    let default_plugin_config = bb_agent_plugins::types::PluginConfig::default();
    let blocklist = bb_agent_plugins::Blocklist::new(0);
    let init_errors = plugin_registry.init_all(&default_plugin_config, &blocklist);
    if !init_errors.is_empty() {
        for err in &init_errors {
            tracing::warn!(error = %err, "Plugin initialization error");
        }
    }
    tracing::info!(
        active = plugin_registry.active_count(),
        "Plugin registry initialized"
    );

    // Install nftables rules for DNS redirection
    let resolver_port = config.dns.listen_port;
    let agent_uid = platform::current_uid();
    let mut nft_manager = NftablesManager::new(resolver_port, agent_uid);

    #[cfg(unix)]
    match nft_manager.install_rules() {
        Ok(()) => tracing::info!("nftables DNS redirect rules installed"),
        Err(e) => tracing::warn!(error = %e, "Failed to install nftables rules (DNS blocking may be limited)"),
    }

    // Start watchdog
    let (mut watchdog_monitor, watchdog_handle, mut recovery_rx) =
        WatchdogMonitor::new(binary_hash.clone());

    let watchdog_shutdown_rx = shutdown_rx.clone();
    let watchdog_task = tokio::spawn(async move {
        watchdog_monitor.run(watchdog_shutdown_rx).await;
    });

    // Start watchdog ping sender
    let ping_shutdown_rx = shutdown_rx.clone();
    let ping_task = bb_agent_core::tamper::watchdog::spawn_ping_sender(
        watchdog_handle,
        binary_hash,
        ping_shutdown_rx,
    );

    // Handle recovery actions from watchdog
    let event_handle = event_emitter.handle();
    let recovery_task = tokio::spawn(async move {
        while let Some(action) = recovery_rx.recv().await {
            match action {
                bb_agent_core::tamper::watchdog::RecoveryAction::LogTamperEvent => {
                    tracing::warn!("Watchdog: logging tamper event");
                    event_handle.emit(bb_agent_core::events::AgentEvent::tamper_detected(
                        "watchdog",
                        "Missed health pings",
                    ));
                }
                bb_agent_core::tamper::watchdog::RecoveryAction::RestartSubsystem(name) => {
                    tracing::warn!(subsystem = %name, "Watchdog: restart requested");
                }
                bb_agent_core::tamper::watchdog::RecoveryAction::SendTamperAlert => {
                    tracing::error!("Watchdog: sending tamper alert to API");
                }
            }
        }
    });

    // Start heartbeat sender
    let heartbeat_config = HeartbeatConfig::self_tier(device_id.clone(), AGENT_VERSION.to_string());
    let mut heartbeat_sender = HeartbeatSender::new(api_client.clone(), heartbeat_config);
    let heartbeat_shutdown_rx = shutdown_rx.clone();
    let heartbeat_task = tokio::spawn(async move {
        heartbeat_sender.run(heartbeat_shutdown_rx).await;
    });

    // Start event reporter
    // EventStore uses rusqlite which is not Sync, so the reporter runs
    // in a dedicated single-threaded task via spawn_blocking.
    let reporter_db_path = events_db_path.clone();
    let reporter_api = api_client.clone();
    let reporter_device_id = device_id.clone();
    let reporter_shutdown_rx = shutdown_rx.clone();
    let reporter_task = tokio::task::spawn_blocking(move || {
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async move {
            let reporter_store = match EventStore::new(&reporter_db_path) {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!(error = %e, "Failed to open reporter event store");
                    return;
                }
            };
            let mut event_reporter = EventReporter::new(
                reporter_api,
                reporter_device_id,
                EnrollmentTier::SelfEnrolled,
                ReportingConfig::default(),
            );
            event_reporter.run(&reporter_store, reporter_shutdown_rx).await;
        });
    });

    // Start periodic binary integrity check
    let integrity_shutdown_rx = shutdown_rx.clone();
    let integrity_emitter = event_emitter.handle();
    let integrity_task = if let Some(bi) = binary_integrity {
        Some(tokio::spawn(async move {
            bi.run_periodic_check(integrity_shutdown_rx, move |e| {
                tracing::error!(error = %e, "Binary integrity violation!");
                integrity_emitter.emit(bb_agent_core::events::AgentEvent::tamper_detected(
                    "integrity",
                    &e.to_string(),
                ));
            })
            .await;
        }))
    } else {
        None
    };

    // Start nftables rule verification loop
    let nft_shutdown_rx = shutdown_rx.clone();
    let nft_emitter = event_emitter.handle();
    let mut nft_shutdown = nft_shutdown_rx;
    let nft_task = tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(30));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    match nft_manager.verify_and_repair() {
                        Ok(true) => {
                            nft_emitter.emit(bb_agent_core::events::AgentEvent::tamper_detected(
                                "nftables",
                                "DNS redirect rules were removed externally",
                            ));
                        }
                        Ok(false) => {} // Rules intact
                        Err(e) => {
                            tracing::debug!(error = %e, "nftables verification failed");
                        }
                    }
                }
                _ = nft_shutdown.changed() => {
                    break;
                }
            }
        }
    });

    // Emit agent started event
    event_emitter.emit(bb_agent_core::events::AgentEvent {
        id: None,
        event_type: bb_common::enums::EventType::AgentStarted,
        category: bb_common::enums::EventCategory::System,
        severity: bb_common::enums::EventSeverity::Info,
        domain: None,
        plugin_id: "agent".to_string(),
        metadata: serde_json::json!({
            "version": AGENT_VERSION,
            "platform": "linux",
        }),
        timestamp: chrono::Utc::now(),
        reported: false,
    });
    let _ = event_emitter.flush();

    // Notify systemd we are ready
    platform::sd_notify_ready();
    platform::sd_notify_status("Running");

    tracing::info!(
        device_id = %device_id,
        plugins = plugin_registry.active_count(),
        "BetBlocker Agent fully initialized and running"
    );

    // --- Phase 4: Wait for shutdown ---
    shutdown_rx.clone().changed().await.ok();

    tracing::info!("Shutdown signal received, cleaning up");
    platform::sd_notify_stopping();
    platform::sd_notify_status("Stopping");

    // Graceful shutdown: deactivate plugins in reverse order
    let deactivate_errors = plugin_registry.deactivate_all();
    for err in &deactivate_errors {
        tracing::warn!(error = %err, "Plugin deactivation error");
    }

    // Final event flush
    let _ = event_emitter.flush();

    // Wait for tasks to finish (with timeout)
    let _ = tokio::time::timeout(Duration::from_secs(5), async {
        let _ = heartbeat_task.await;
        let _ = reporter_task.await;
        let _ = watchdog_task.await;
        let _ = ping_task.await;
        let _ = recovery_task.await;
        let _ = nft_task.await;
        if let Some(task) = integrity_task {
            let _ = task.await;
        }
    })
    .await;

    tracing::info!("BetBlocker Agent shutdown complete");
    Ok(())
}
