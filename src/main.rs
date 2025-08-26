#[tokio::main]
async fn main() {
    env_logger::init();

    let result = fluent_cli::cli::run_modular().await;
    if let Err(err) = result {
        let code = classify_exit_code(&err);
        eprintln!("{}", sanitize_error_message(&err));
        std::process::exit(code);
    }
}

fn sanitize_error_message(err: &anyhow::Error) -> String {
    let msg = format!("{}", err);
    fluent_core::redaction::redact_secrets_in_text(&msg)
}

fn classify_exit_code(err: &anyhow::Error) -> i32 {
    // First, look for typed CLI errors
    if let Some(cli_err) = err.downcast_ref::<fluent_cli::error::CliError>() {
        return match cli_err {
            fluent_cli::error::CliError::ArgParse(_) => 2,
            fluent_cli::error::CliError::Config(_) => 10,
            fluent_cli::error::CliError::Engine(_) => 13,
            fluent_cli::error::CliError::Network(_) => 12,
            fluent_cli::error::CliError::Validation(_) => 14,
            fluent_cli::error::CliError::Unknown(_) => 1,
        };
    }

    // Map core error types if present
    if let Some(core_err) = err.downcast_ref::<fluent_core::error::FluentError>() {
        return match core_err {
            fluent_core::error::FluentError::Config(_) => 10,
            fluent_core::error::FluentError::Auth(_) => 11,
            fluent_core::error::FluentError::Network(_) => 12,
            fluent_core::error::FluentError::Engine(_) => 13,
            fluent_core::error::FluentError::Validation(_) => 14,
            fluent_core::error::FluentError::File(_) => 15,
            fluent_core::error::FluentError::Storage(_) => 16,
            fluent_core::error::FluentError::Pipeline(_) => 17,
            fluent_core::error::FluentError::Cache(_) => 18,
            fluent_core::error::FluentError::LockTimeout(_) => 19,
            fluent_core::error::FluentError::Cost(_) => 21,
            fluent_core::error::FluentError::Internal(_) => 20,
        };
    }

    // Reqwest network errors
    if err.downcast_ref::<reqwest::Error>().is_some() {
        return 12;
    }

    // Default unknown error
    1
}
