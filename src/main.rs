#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    // Use the modular CLI from fluent_cli
    fluent_cli::cli::run_modular().await
}
