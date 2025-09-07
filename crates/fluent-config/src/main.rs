use fluent_engines::config_cli::ConfigCli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = env_logger::try_init();
    ConfigCli::run().await
}

