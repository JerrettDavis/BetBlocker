use bb_api::config::ApiConfig;
use bb_api::routes;
use bb_api::state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env file if present
    let _ = dotenvy::dotenv();

    // Initialize tracing with JSON formatting and env filter
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .json()
        .init();

    tracing::info!("BetBlocker API starting...");

    // Load configuration
    let config = ApiConfig::from_env()?;
    let host = config.host.clone();
    let port = config.port;

    // Build application state
    let state = AppState::new(config).await?;

    // Note: migrations use Flyway-style naming (V001__...) and should be run
    // via an external migration tool. The sqlx::migrate! macro expects a different
    // naming convention, so we skip auto-migration here.
    tracing::info!("Database migrations should be applied externally (Flyway-style naming).");

    // Build router
    let app = routes::router(state);

    // Bind and serve
    let addr = format!("{host}:{port}");
    tracing::info!("Listening on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("Server shut down gracefully.");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
}
