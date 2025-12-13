use fluent_engines::config_cli::ConfigCli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging using centralized logging module
    fluent_core::logging::init_logging();
    ConfigCli::run().await
}
