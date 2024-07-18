use fluent_cli::cli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    cli::run().await
}


