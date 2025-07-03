use fluent_cli::cli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    cli::run().await
}
