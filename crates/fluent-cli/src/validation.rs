
use clap::ArgMatches;
use fluent_core::error::{FluentError, FluentResult, ValidationError};
use fluent_core::input_validator::InputValidator;
use std::path::Path;

/// Convert anyhow errors to FluentError with context
pub fn to_fluent_error(err: anyhow::Error, context: &str) -> FluentError {
    FluentError::Internal(format!("{}: {}", context, err))
}

/// Validate required CLI arguments
pub fn validate_required_string(
    matches: &ArgMatches,
    arg_name: &str,
    context: &str,
) -> FluentResult<String> {
    matches
        .get_one::<String>(arg_name)
        .ok_or_else(|| {
            FluentError::Validation(ValidationError::MissingField(format!(
                "{} is required for {}",
                arg_name, context
            )))
        })
        .map(|s| s.clone())
}

/// Validate file path with security checks
pub fn validate_file_path_secure(path: &str, context: &str) -> FluentResult<String> {
    if path.is_empty() {
        return Err(FluentError::Validation(ValidationError::InvalidFormat {
            input: path.to_string(),
            expected: format!("Non-empty file path for {}", context),
        }));
    }

    // Security check: prevent path traversal
    if path.contains("..") || path.contains("~") {
        return Err(FluentError::Validation(ValidationError::DangerousPattern(
            "Path traversal detected".to_string(),
        )));
    }

    // Validate path exists and is readable
    if !Path::new(path).exists() {
        return Err(FluentError::Validation(ValidationError::InvalidFormat {
            input: path.to_string(),
            expected: "Existing file path".to_string(),
        }));
    }

    Ok(path.to_string())
}

/// Validate request payload with security checks
pub fn validate_request_payload(payload: &str, context: &str) -> FluentResult<String> {
    match InputValidator::validate_request_payload(payload) {
        Ok(validated) => Ok(validated),
        Err(e) => Err(FluentError::Validation(ValidationError::InvalidFormat {
            input: payload.to_string(),
            expected: format!("Valid payload for {}: {}", context, e),
        })),
    }
}

/// Validate numeric parameters within acceptable ranges
pub fn validate_numeric_parameter(
    value: u32,
    min: u32,
    max: u32,
    param_name: &str,
) -> FluentResult<u32> {
    if value < min || value > max {
        return Err(FluentError::Validation(ValidationError::InvalidFormat {
            input: value.to_string(),
            expected: format!("{} must be between {} and {}", param_name, min, max),
        }));
    }
    Ok(value)
}

/// Validate engine name against allowed engines
pub fn validate_engine_name(engine_name: &str) -> FluentResult<String> {
    if engine_name.is_empty() {
        return Err(FluentError::Validation(ValidationError::MissingField(
            "Engine name cannot be empty".to_string(),
        )));
    }

    let allowed_engines = [
        "openai",
        "anthropic",
        "google",
        "cohere",
        "mistral",
        "perplexity",
        "groq",
        "replicate",
        "stabilityai",
        "leonardoai",
        "dalle",
        "webhook",
        "flowise",
        "langflow",
    ];

    if !allowed_engines.contains(&engine_name) {
        return Err(FluentError::Validation(ValidationError::InvalidFormat {
            input: engine_name.to_string(),
            expected: format!("One of: {}", allowed_engines.join(", ")),
        }));
    }

    Ok(engine_name.to_string())
}

/// Parse key-value pairs from command line arguments
pub fn parse_key_value_pair(s: &str) -> Option<(String, String)> {
    if let Some((key, value)) = s.split_once('=') {
        Some((key.to_string(), value.to_string()))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_engine_name() {
        assert!(validate_engine_name("openai").is_ok());
        assert!(validate_engine_name("anthropic").is_ok());
        assert!(validate_engine_name("invalid_engine").is_err());
        assert!(validate_engine_name("").is_err());
    }

    #[test]
    fn test_validate_numeric_parameter() {
        assert!(validate_numeric_parameter(50, 1, 100, "test").is_ok());
        assert!(validate_numeric_parameter(0, 1, 100, "test").is_err());
        assert!(validate_numeric_parameter(101, 1, 100, "test").is_err());
    }

    #[test]
    fn test_parse_key_value_pair() {
        assert_eq!(
            parse_key_value_pair("key=value"),
            Some(("key".to_string(), "value".to_string()))
        );
        assert_eq!(parse_key_value_pair("invalid"), None);
    }

    #[test]
    fn test_validate_file_path_secure() {
        // Test path traversal detection
        assert!(validate_file_path_secure("../etc/passwd", "test").is_err());
        assert!(validate_file_path_secure("~/secret", "test").is_err());
        
        // Test empty path
        assert!(validate_file_path_secure("", "test").is_err());
    }
}
