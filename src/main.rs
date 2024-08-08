use std::process;
use fluent_cli::cli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    if let Err(e) = cli::run().await {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
    // Ensure the program exits even if run() completes without error
    process::exit(0);
}


