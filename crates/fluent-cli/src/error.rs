use thiserror::Error;

#[derive(Debug, Error)]
pub enum CliError {
    #[error("argument parsing error: {0}")]
    ArgParse(String),
    #[error("configuration error: {0}")]
    Config(String),
    #[error("engine error: {0}")]
    Engine(String),
    #[error("network error: {0}")]
    Network(String),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("unknown error: {0}")]
    Unknown(String),
}
