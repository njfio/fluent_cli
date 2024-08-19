#[tokio::main]
async fn main() -> anyhow::Result<()> {
    fluent_cli::v2::run().await
}
