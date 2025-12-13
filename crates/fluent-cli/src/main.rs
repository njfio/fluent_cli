use fluent_cli::cli;
use fluent_cli::exit_codes;

#[tokio::main]
async fn main() {
    // Initialize logging using centralized logging module
    let req_id = fluent_core::logging::init_cli_logging();
    tracing::info!(request_id = %req_id, "fluent-cli startup");

    // Run the CLI and handle errors with proper exit codes
    match cli::run_modular().await {
        Ok(_) => {
            tracing::info!(request_id = %req_id, "fluent-cli completed successfully");
            std::process::exit(exit_codes::SUCCESS);
        }
        Err(e) => {
            let exit_code = exit_codes::anyhow_error_to_exit_code(&e);

            // Log the error with structured logging
            tracing::error!(
                request_id = %req_id,
                error = %e,
                exit_code = exit_code,
                "fluent-cli terminated with error"
            );

            // Print error to stderr for user visibility
            eprintln!("Error: {}", e);

            std::process::exit(exit_code);
        }
    }
}
