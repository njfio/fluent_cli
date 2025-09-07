use fluent_cli::cli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging similar to root binary
    let log_fmt = std::env::var("FLUENT_LOG_FORMAT").unwrap_or_default();
    if log_fmt.eq_ignore_ascii_case("json") {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")))
            .json()
            .try_init();
    } else {
        let _ = env_logger::try_init();
    }

    // Attach request id
    let req_id = uuid::Uuid::new_v4().to_string();
    std::env::set_var("FLUENT_REQUEST_ID", &req_id);
    tracing::info!(request_id = %req_id, "fluent-cli startup");

    cli::run_modular().await
}
