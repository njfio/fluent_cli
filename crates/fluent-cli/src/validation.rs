
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

/// Validate engine name against allowed engines (configurable)
pub fn validate_engine_name(engine_name: &str) -> FluentResult<String> {
    if engine_name.is_empty() {
        return Err(FluentError::Validation(ValidationError::MissingField(
            "Engine name cannot be empty".to_string(),
        )));
    }

    // Normalize engine name to lowercase for comparison
    let normalized_engine = engine_name.to_lowercase();

    // Get allowed engines from configuration or environment
    let allowed_engines = get_allowed_engines();

    if !allowed_engines.contains(&normalized_engine) {
        return Err(FluentError::Validation(ValidationError::InvalidFormat {
            input: engine_name.to_string(),
            expected: format!("One of: {}", allowed_engines.join(", ")),
        }));
    }

    // Return the original case for consistency
    Ok(engine_name.to_string())
}

/// Get allowed engines from configuration file or environment variables
fn get_allowed_engines() -> Vec<String> {
    // First, try to get from environment variable
    if let Ok(engines_env) = std::env::var("FLUENT_ALLOWED_ENGINES") {
        let engines: Vec<String> = engines_env
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();

        if !engines.is_empty() {
            return engines;
        }
    }

    // Try to load from configuration file
    if let Ok(config_engines) = load_engines_from_config() {
        if !config_engines.is_empty() {
            return config_engines;
        }
    }

    // Fallback to default engines
    get_default_engines()
}

/// Load allowed engines from configuration file
fn load_engines_from_config() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    use std::fs;
    use std::path::Path;

    // Look for engine configuration in common locations
    let config_paths = [
        "fluent_engines.json",
        "config/fluent_engines.json",
        ".fluent/engines.json",
        std::env::var("HOME").unwrap_or_default() + "/.fluent/engines.json",
    ];

    for config_path in &config_paths {
        if Path::new(config_path).exists() {
            let content = fs::read_to_string(config_path)?;

            // Try to parse as JSON
            if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(engines_array) = json_value.get("allowed_engines") {
                    if let Some(engines) = engines_array.as_array() {
                        let engine_names: Vec<String> = engines
                            .iter()
                            .filter_map(|v| v.as_str())
                            .map(|s| s.to_lowercase())
                            .collect();

                        if !engine_names.is_empty() {
                            return Ok(engine_names);
                        }
                    }
                }
            }
        }
    }

    Err("No valid engine configuration found".into())
}

/// Get default allowed engines
fn get_default_engines() -> Vec<String> {
    vec![
        "openai".to_string(),
        "anthropic".to_string(),
        "google".to_string(),
        "cohere".to_string(),
        "mistral".to_string(),
        "perplexity".to_string(),
        "groq".to_string(),
        "replicate".to_string(),
        "stabilityai".to_string(),
        "leonardoai".to_string(),
        "dalle".to_string(),
        "webhook".to_string(),
        "flowise".to_string(),
        "langflow".to_string(),
        "gemini".to_string(),
        "claude".to_string(),
        "gpt-4".to_string(),
        "gpt-4o".to_string(),
        "sonnet3.5".to_string(),
        "gemini-flash".to_string(),
    ]
}

// Re-export the centralized parse_key_value_pair function
pub use fluent_core::config::parse_key_value_pair;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_engine_name() {
        // Test valid engines from default list
        assert!(validate_engine_name("openai").is_ok());
        assert!(validate_engine_name("anthropic").is_ok());
        assert!(validate_engine_name("google").is_ok());

        // Test case insensitivity (should work since we normalize to lowercase)
        assert!(validate_engine_name("OpenAI").is_ok());
        assert!(validate_engine_name("ANTHROPIC").is_ok());

        // Test invalid engines
        assert!(validate_engine_name("invalid_engine").is_err());
        assert!(validate_engine_name("").is_err());
    }

    #[test]
    fn test_get_default_engines() {
        let engines = get_default_engines();
        assert!(engines.contains(&"openai".to_string()));
        assert!(engines.contains(&"anthropic".to_string()));
        assert!(engines.contains(&"google".to_string()));
        assert!(engines.len() > 10); // Should have a reasonable number of engines
    }

    #[test]
    fn test_environment_variable_engine_validation() {
        // Set environment variable for testing
        std::env::set_var("FLUENT_ALLOWED_ENGINES", "custom_engine,another_engine");

        // The validation should now accept custom engines
        let allowed = get_allowed_engines();
        assert!(allowed.contains(&"custom_engine".to_string()));
        assert!(allowed.contains(&"another_engine".to_string()));

        // Clean up
        std::env::remove_var("FLUENT_ALLOWED_ENGINES");
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
