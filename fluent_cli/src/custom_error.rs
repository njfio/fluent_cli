use std::fmt;
use std::error::Error;
use serde_json::error::Error as JsonError;
use duckdb::Error as DuckDbError;
use reqwest::Error as ReqwestError;
use serde::ser::Error as SerdeError;

#[derive(Debug, Clone)]
pub enum FluentError {
    JsonError(String),
    DuckDbError(String),
    HttpError(String),
    IoError(String),
    CustomError(String),
}

impl fmt::Display for FluentError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FluentError::JsonError(e) => write!(f, "JSON error: {}", e),
            FluentError::DuckDbError(e) => write!(f, "DuckDB error: {}", e),
            FluentError::HttpError(e) => write!(f, "HTTP error: {}", e),
            FluentError::IoError(e) => write!(f, "I/O error: {}", e),
            FluentError::CustomError(e) => write!(f, "Custom error: {}", e),
        }
    }
}

impl Error for FluentError {}

impl From<JsonError> for FluentError {
    fn from(error: JsonError) -> Self {
        FluentError::JsonError(error.to_string())
    }
}

impl From<DuckDbError> for FluentError {
    fn from(error: DuckDbError) -> Self {
        FluentError::DuckDbError(error.to_string())
    }
}

impl From<ReqwestError> for FluentError {
    fn from(error: ReqwestError) -> Self {
        FluentError::HttpError(error.to_string())
    }
}

impl From<std::io::Error> for FluentError {
    fn from(error: std::io::Error) -> Self {
        FluentError::IoError(error.to_string())
    }
}

// Add this new implementation
impl From<FluentError> for JsonError {
    fn from(error: FluentError) -> Self {
        JsonError::custom(error.to_string())
    }
}

pub type FluentResult<T> = std::result::Result<T, FluentError>;