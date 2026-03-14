//! BetBlocker Agent for Windows.

mod dns_redirect;
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
use bb_agent_core::config::AgentConfig;
use bb_agent_core::events::emitter::EventEmitter;
use bb_agent_core::events::store::EventStore;
use bb_agent_core::tamper::integrity::BinaryIntegrity;
use bb_agent_core::tamper::watchdog::WatchdogMonitor;
use bb_common::enums::EnrollmentTier;
use bb_common::models::ReportingConfig;

use dns_redirect::WindowsDnsRedirect;

const AGENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const SERVICE_NAME: &str = "BetBlockerAgent";

/// BetBlocker Agent for Windows — gambling site blocking service.
#[derive(Parser, Debug)]
#[command(name = "bb-agent-windows", version, about)]
pub struct Cli {
    /// Path to the configuration directory.
    #[arg(long, default_value = r"C:\ProgramData\BetBlocker")]
    config_dir: PathBuf,

    /// Enrollment token for initial device registration.
    #[arg(long)]
    enroll: Option<String>,

    /// Path to the configuration file.
    #[arg(long)]
    config: Option<PathBuf>,

    /// Install the Windows service and exit.
    #[arg(long)]
    install_service: bool,

    /// Uninstall the Windows service and exit.
    #[arg(long)]
    uninstall_service: bool,
}

// ---------------------------------------------------------------------------
// Tracing setup
// ---------------------------------------------------------------------------

/// Initialise tracing: stderr output for interactive use; on Windows this
/// can be supplemented with Windows Event Log via an appender, but for
/// portability we always enable the fmt subscriber.
fn setup_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(true)
        .with_thread_ids(true)
        .init();
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    setup_tracing();

    let cli = Cli::parse();

    tracing::info!(
        version = AGENT_VERSION,
        config_dir = %cli.config_dir.display(),
        "BetBlocker Windows Agent starting"
    );

    // --install-service / --uninstall-service are handled before async runtime
    if cli.install_service {
        handle_install_service(&cli);
        return;
    }

    if cli.uninstall_service {
        handle_uninstall_service();
        return;
    }

    if let Err(e) = run(cli).await {
        tracing::error!(error = %e, "Agent failed");
        std::process::exit(1);
    }
}

// ---------------------------------------------------------------------------
// Service install / uninstall helpers
// ---------------------------------------------------------------------------

fn handle_install_service(cli: &Cli) {
    use bb_shim_windows::service::{ServiceConfig, register_service, set_failure_actions};

    let binary_path = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("bb-agent-windows.exe"));

    let config = ServiceConfig::new(
        SERVICE_NAME,
        "BetBlocker Agent",
        "BetBlocker gambling site blocking service",
        binary_path,
    );

    match register_service(&config) {
        Ok(()) => {
            tracing::info!("Service registered successfully");
            // Configure automatic restart on failure
            let _ = set_failure_actions(SERVICE_NAME);
            tracing::info!(service = SERVICE_NAME, "Windows service installed");
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to register service");
            std::process::exit(1);
        }
    }
    let _ = cli; // used for binary_path above indirectly
}

fn handle_uninstall_service() {
    use bb_shim_windows::service::unregister_service;

    match unregister_service(SERVICE_NAME) {
        Ok(()) => tracing::info!(service = SERVICE_NAME, "Windows service uninstalled"),
        Err(e) => {
            tracing::error!(error = %e, "Failed to unregister service");
            std::process::exit(1);
        }
    }
}

// ---------------------------------------------------------------------------
// Core agent run loop
// ---------------------------------------------------------------------------

/// Run the agent with the given CLI arguments.
///
/// This mirrors the Linux agent pattern: setup, registration, subsystems,
/// wait for shutdown.
pub async fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    // --- Phase 1: Setup ---

    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // Windows Ctrl+C handler (also handles SCM STOP when running as service)
    {
        let shutdown_tx_clone = shutdown_tx.clone();
        tokio::spawn(async move {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to register Ctrl+C handler");
            tracing::info!("Received shutdown signal, initiating shutdown");
            let _ = shutdown_tx_clone.send(true);
        });
    }

    // Ensure data directories exist
    platform::ensure_directories().map_err(|e| format!("Failed to create directories: {e}"))?;

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

    // Initialise event store
    let events_db_path = config.data_dir.join("events.db");
    let event_store = EventStore::new(&events_db_path)
        .map_err(|e| format!("Failed to open event store: {e}"))?;
    let event_emitter = EventEmitter::new(event_store);

    // Initialise certificate store
    let cert_store = Arc::new(
        FileCertificateStore::new(&config.data_dir)
            .map_err(|e| format!("Failed to init cert store: {e}"))?,
    );

    // Binary integrity
    let binary_integrity = match BinaryIntegrity::new(None) {
        Ok(bi) => {
            tracing::info!(hash = bi.startup_hash_hex(), "Binary integrity check passed");
            Some(bi)
        }
        Err(e) => {
            tracing::warn!(error = %e, "Binary integrity check skipped");
            None
        }
    };

    let binary_hash = binary_integrity
        .as_ref()
        .map(|bi| bi.startup_hash().to_vec())
        .unwrap_or_default();

    // --- Phase 2: Registration ---

    let ca_pem = cert_store.load_ca_chain().ok().flatten();

    let device_id = if let Some(enrollment_token) = &cli.enroll {
        tracing::info!("Starting device enrollment");
        let api_client = Arc::new(ApiClient::new_insecure(config.api_url.clone()));
        let reg_service = RegistrationService::new(api_client.clone(), cert_store.clone());
        let result = reg_service
            .register(enrollment_token, AGENT_VERSION)
            .await
            .map_err(|e| format!("Registration failed: {e}"))?;
        tracing::info!(device_id = %result.device_id, "Device registered successfully");
        result.device_id
    } else if let Some(id) = &config.device_id {
        tracing::info!(device_id = %id, "Using existing device ID from config");
        id.clone()
    } else {
        return Err("No device ID and no enrollment token. Run with --enroll <token>".into());
    };

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

    // --- Phase 3: Initialise subsystems ---

    // Plugin registry
    let mut plugin_registry = bb_agent_plugins::PluginRegistry::with_defaults();
    let default_plugin_config = bb_agent_plugins::types::PluginConfig::default();
    let blocklist = bb_agent_plugins::Blocklist::new(0);
    let init_errors = plugin_registry.init_all(&default_plugin_config, &blocklist);
    for err in &init_errors {
        tracing::warn!(error = %err, "Plugin initialization error");
    }
    tracing::info!(active = plugin_registry.active_count(), "Plugin registry initialized");

    // DNS redirect (Windows Firewall rules)
    let resolver_port = config.dns.listen_port;
    let mut dns_redirect = WindowsDnsRedirect::new(resolver_port);
    match dns_redirect.install_rules() {
        Ok(()) => tracing::info!("Windows DNS redirect firewall rules installed"),
        Err(e) => tracing::warn!(error = %e, "Failed to install DNS redirect rules"),
    }

    // Watchdog
    let (mut watchdog_monitor, watchdog_handle, mut recovery_rx) =
        WatchdogMonitor::new(binary_hash.clone());

    let watchdog_task = {
        let watchdog_shutdown_rx = shutdown_rx.clone();
        tokio::spawn(async move { watchdog_monitor.run(watchdog_shutdown_rx).await })
    };

    let ping_task = bb_agent_core::tamper::watchdog::spawn_ping_sender(
        watchdog_handle,
        binary_hash,
        shutdown_rx.clone(),
    );

    let recovery_task = {
        let event_handle = event_emitter.handle();
        tokio::spawn(async move {
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
        })
    };

    // Heartbeat
    let heartbeat_task = {
        let heartbeat_config =
            HeartbeatConfig::self_tier(device_id.clone(), AGENT_VERSION.to_string());
        let mut heartbeat_sender = HeartbeatSender::new(api_client.clone(), heartbeat_config);
        let hb_shutdown = shutdown_rx.clone();
        tokio::spawn(async move { heartbeat_sender.run(hb_shutdown).await })
    };

    // Event reporter (spawn_blocking for !Sync EventStore)
    let reporter_task = {
        let reporter_db_path = events_db_path.clone();
        let reporter_api = api_client.clone();
        let reporter_device_id = device_id.clone();
        let reporter_shutdown_rx = shutdown_rx.clone();
        tokio::task::spawn_blocking(move || {
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
                event_reporter
                    .run(&reporter_store, reporter_shutdown_rx)
                    .await;
            });
        })
    };

    // Binary integrity periodic check
    let integrity_task = if let Some(bi) = binary_integrity {
        let integrity_shutdown_rx = shutdown_rx.clone();
        let integrity_emitter = event_emitter.handle();
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

    // DNS rule verification loop
    let dns_verify_task = {
        let event_handle = event_emitter.handle();
        let mut dns_shutdown = shutdown_rx.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(30));
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        match dns_redirect.verify_and_repair() {
                            Ok(false) => {
                                event_handle.emit(
                                    bb_agent_core::events::AgentEvent::tamper_detected(
                                        "dns_redirect",
                                        "DNS redirect firewall rules were removed externally",
                                    ),
                                );
                            }
                            Ok(true) => {}
                            Err(e) => {
                                tracing::debug!(error = %e, "DNS rule verification failed");
                            }
                        }
                    }
                    _ = dns_shutdown.changed() => break,
                }
            }
        })
    };

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
            "platform": "windows",
        }),
        timestamp: chrono::Utc::now(),
        reported: false,
    });
    let _ = event_emitter.flush();

    // Notify SCM we are ready
    platform::service_notify_ready();
    platform::service_notify_status("Running");

    tracing::info!(
        device_id = %device_id,
        plugins = plugin_registry.active_count(),
        "BetBlocker Windows Agent fully initialized and running"
    );

    // --- Phase 4: Wait for shutdown ---
    shutdown_rx.clone().changed().await.ok();

    tracing::info!("Shutdown signal received, cleaning up");
    platform::service_notify_stopping();
    platform::service_notify_status("Stopping");

    // Deactivate plugins
    let deactivate_errors = plugin_registry.deactivate_all();
    for err in &deactivate_errors {
        tracing::warn!(error = %err, "Plugin deactivation error");
    }

    let _ = event_emitter.flush();

    // Wait for tasks
    let _ = tokio::time::timeout(Duration::from_secs(5), async {
        let _ = heartbeat_task.await;
        let _ = reporter_task.await;
        let _ = watchdog_task.await;
        let _ = ping_task.await;
        let _ = recovery_task.await;
        let _ = dns_verify_task.await;
        if let Some(task) = integrity_task {
            let _ = task.await;
        }
    })
    .await;

    tracing::info!("BetBlocker Windows Agent shutdown complete");
    Ok(())
}

// ---------------------------------------------------------------------------
// Integration tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that a Cli struct can be constructed with the expected defaults.
    #[test]
    fn cli_defaults() {
        let cli = Cli {
            config_dir: PathBuf::from(r"C:\ProgramData\BetBlocker"),
            enroll: None,
            config: None,
            install_service: false,
            uninstall_service: false,
        };
        assert_eq!(cli.config_dir, PathBuf::from(r"C:\ProgramData\BetBlocker"));
        assert!(!cli.install_service);
        assert!(!cli.uninstall_service);
    }

    /// Smoke-test that the agent run loop exits cleanly when shutdown is
    /// pre-signalled (no network calls needed).
    ///
    /// Note: This test creates a `Cli` with a temp config dir so that the
    /// missing `agent.toml` defaults are accepted and the registration path
    /// falls through to the "no device ID" early exit.
    #[tokio::test]
    async fn run_exits_on_no_device_id() {
        let tmp = std::env::temp_dir().join("bb-agent-windows-test");
        let cli = Cli {
            config_dir: tmp.clone(),
            enroll: None,
            config: Some(tmp.join("nonexistent.toml")),
            install_service: false,
            uninstall_service: false,
        };
        // Without a device ID or enroll token the run function returns an error.
        let result = run(cli).await;
        assert!(result.is_err(), "Expected error without device ID");
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("enrollment") || msg.contains("device"), "Unexpected error: {msg}");
    }
}
