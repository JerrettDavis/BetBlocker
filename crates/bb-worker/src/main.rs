#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("BetBlocker Worker starting...");
    // TODO: Background job processing
}
