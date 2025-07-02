use anyhow::{anyhow, Context, Result};
use fluent_core::config::{EngineConfig, ConnectionConfig, Neo4jConfig};
use fluent_core::input_validator::InputValidator;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Enhanced configuration with validation and environment support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedEngineConfig {
    #[serde(flatten)]
    pub base: EngineConfig,
    
    /// Configuration metadata
    pub metadata: ConfigMetadata,
    
    /// Validation rules
    pub validation: ValidationRules,
    
    /// Environment-specific overrides
    pub environments: HashMap<String, EnvironmentOverrides>,
}

/// Configuration metadata for tracking and management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigMetadata {
    pub version: String,
    pub created_at: String,
    pub updated_at: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub owner: Option<String>,
}

/// Validation rules for configuration parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRules {
    pub required_parameters: Vec<String>,
    pub parameter_types: HashMap<String, ParameterType>,
    pub parameter_constraints: HashMap<String, ParameterConstraints>,
    pub connection_timeout: Option<u64>,
    pub request_timeout: Option<u64>,
}

/// Parameter type definitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParameterType {
    String,
    Number,
    Boolean,
    Array,
    Object,
    SecretString, // For sensitive data
}

/// Parameter constraints for validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterConstraints {
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    pub allowed_values: Option<Vec<Value>>,
    pub pattern: Option<String>, // Regex pattern
}

/// Environment-specific configuration overrides
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentOverrides {
    pub parameters: HashMap<String, Value>,
    pub connection: Option<ConnectionConfig>,
    pub neo4j: Option<Neo4jConfig>,
}

/// Configuration manager with caching and validation
pub struct ConfigManager {
    configs: Arc<RwLock<HashMap<String, EnhancedEngineConfig>>>,
    pub config_dir: PathBuf,
    current_environment: String,
}

impl ConfigManager {
    /// Create a new configuration manager
    pub fn new(config_dir: PathBuf) -> Self {
        let current_environment = env::var("FLUENT_ENV").unwrap_or_else(|_| "development".to_string());
        
        Self {
            configs: Arc::new(RwLock::new(HashMap::new())),
            config_dir,
            current_environment,
        }
    }

    /// Load configuration from file with validation
    pub async fn load_config(&self, engine_name: &str) -> Result<EngineConfig> {
        // Check cache first
        {
            let configs = self.configs.read().await;
            if let Some(enhanced_config) = configs.get(engine_name) {
                return self.apply_environment_overrides(&enhanced_config.base, &enhanced_config.environments);
            }
        }

        // Load from file
        let config_path = self.config_dir.join(format!("{}.json", engine_name));
        let enhanced_config = self.load_from_file(&config_path).await?;

        // Validate configuration
        self.validate_config(&enhanced_config).await?;

        // Cache the configuration
        {
            let mut configs = self.configs.write().await;
            configs.insert(engine_name.to_string(), enhanced_config.clone());
        }

        // Apply environment overrides
        self.apply_environment_overrides(&enhanced_config.base, &enhanced_config.environments)
    }

    /// Save configuration to file
    pub async fn save_config(&self, engine_name: &str, config: &EnhancedEngineConfig) -> Result<()> {
        // Validate before saving
        self.validate_config(config).await?;

        // Update metadata
        let mut config_to_save = config.clone();
        config_to_save.metadata.updated_at = chrono::Utc::now().to_rfc3339();

        // Save to file
        let config_path = self.config_dir.join(format!("{}.json", engine_name));
        let json = serde_json::to_string_pretty(&config_to_save)?;
        tokio::fs::write(&config_path, json).await?;

        // Update cache
        {
            let mut configs = self.configs.write().await;
            configs.insert(engine_name.to_string(), config_to_save);
        }

        Ok(())
    }

    /// Create a new configuration with defaults
    pub fn create_default_config(engine_type: &str, name: &str) -> EnhancedEngineConfig {
        let base_config = match engine_type {
            "openai" => Self::create_openai_config(name),
            "anthropic" => Self::create_anthropic_config(name),
            "google_gemini" => Self::create_gemini_config(name),
            _ => Self::create_generic_config(engine_type, name),
        };

        EnhancedEngineConfig {
            base: base_config,
            metadata: ConfigMetadata {
                version: "1.0.0".to_string(),
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
                description: Some(format!("Configuration for {} engine", engine_type)),
                tags: vec![engine_type.to_string()],
                owner: env::var("USER").ok(),
            },
            validation: Self::create_validation_rules(engine_type),
            environments: HashMap::new(),
        }
    }

    /// Validate configuration against rules
    pub async fn validate_config(&self, config: &EnhancedEngineConfig) -> Result<()> {
        // Validate required parameters
        for required_param in &config.validation.required_parameters {
            if !config.base.parameters.contains_key(required_param) {
                return Err(anyhow!("Required parameter '{}' is missing", required_param));
            }
        }

        // Validate parameter types and constraints
        for (param_name, param_value) in &config.base.parameters {
            if let Some(param_type) = config.validation.parameter_types.get(param_name) {
                self.validate_parameter_type(param_name, param_value, param_type)?;
            }

            if let Some(constraints) = config.validation.parameter_constraints.get(param_name) {
                self.validate_parameter_constraints(param_name, param_value, constraints)?;
            }
        }

        // Validate connection configuration
        InputValidator::validate_url_components(
            &config.base.connection.protocol,
            &config.base.connection.hostname,
            config.base.connection.port,
            &config.base.connection.request_path,
        )?;

        Ok(())
    }

    /// List all available configurations
    pub async fn list_configs(&self) -> Result<Vec<String>> {
        let mut configs = Vec::new();
        
        let mut entries = tokio::fs::read_dir(&self.config_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            if let Some(file_name) = entry.file_name().to_str() {
                if file_name.ends_with(".json") {
                    let config_name = file_name.trim_end_matches(".json");
                    configs.push(config_name.to_string());
                }
            }
        }

        Ok(configs)
    }

    /// Get configuration metadata
    pub async fn get_metadata(&self, engine_name: &str) -> Result<ConfigMetadata> {
        let configs = self.configs.read().await;
        if let Some(config) = configs.get(engine_name) {
            Ok(config.metadata.clone())
        } else {
            // Load from file to get metadata
            let config_path = self.config_dir.join(format!("{}.json", engine_name));
            let enhanced_config = self.load_from_file(&config_path).await?;
            Ok(enhanced_config.metadata)
        }
    }

    /// Update configuration parameters
    pub async fn update_parameters(&self, engine_name: &str, updates: HashMap<String, Value>) -> Result<()> {
        let mut enhanced_config = {
            let configs = self.configs.read().await;
            configs.get(engine_name)
                .ok_or_else(|| anyhow!("Configuration '{}' not found", engine_name))?
                .clone()
        };

        // Apply updates
        for (key, value) in updates {
            enhanced_config.base.parameters.insert(key, value);
        }

        // Save updated configuration
        self.save_config(engine_name, &enhanced_config).await
    }

    // Private helper methods

    async fn load_from_file(&self, path: &Path) -> Result<EnhancedEngineConfig> {
        let content = tokio::fs::read_to_string(path).await
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        
        serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))
    }

    fn apply_environment_overrides(
        &self,
        base_config: &EngineConfig,
        environments: &HashMap<String, EnvironmentOverrides>,
    ) -> Result<EngineConfig> {
        let mut config = base_config.clone();

        if let Some(env_overrides) = environments.get(&self.current_environment) {
            // Apply parameter overrides
            for (key, value) in &env_overrides.parameters {
                config.parameters.insert(key.clone(), value.clone());
            }

            // Apply connection overrides
            if let Some(connection_override) = &env_overrides.connection {
                config.connection = connection_override.clone();
            }

            // Apply Neo4j overrides
            if let Some(neo4j_override) = &env_overrides.neo4j {
                config.neo4j = Some(neo4j_override.clone());
            }
        }

        Ok(config)
    }

    fn validate_parameter_type(&self, name: &str, value: &Value, param_type: &ParameterType) -> Result<()> {
        let is_valid = match param_type {
            ParameterType::String | ParameterType::SecretString => value.is_string(),
            ParameterType::Number => value.is_number(),
            ParameterType::Boolean => value.is_boolean(),
            ParameterType::Array => value.is_array(),
            ParameterType::Object => value.is_object(),
        };

        if !is_valid {
            return Err(anyhow!("Parameter '{}' has invalid type. Expected: {:?}", name, param_type));
        }

        Ok(())
    }

    fn validate_parameter_constraints(&self, name: &str, value: &Value, constraints: &ParameterConstraints) -> Result<()> {
        // Validate numeric constraints
        if let Some(num) = value.as_f64() {
            if let Some(min) = constraints.min_value {
                if num < min {
                    return Err(anyhow!("Parameter '{}' value {} is below minimum {}", name, num, min));
                }
            }
            if let Some(max) = constraints.max_value {
                if num > max {
                    return Err(anyhow!("Parameter '{}' value {} is above maximum {}", name, num, max));
                }
            }
        }

        // Validate string constraints
        if let Some(string) = value.as_str() {
            if let Some(min_len) = constraints.min_length {
                if string.len() < min_len {
                    return Err(anyhow!("Parameter '{}' length {} is below minimum {}", name, string.len(), min_len));
                }
            }
            if let Some(max_len) = constraints.max_length {
                if string.len() > max_len {
                    return Err(anyhow!("Parameter '{}' length {} is above maximum {}", name, string.len(), max_len));
                }
            }
        }

        // Validate allowed values
        if let Some(allowed) = &constraints.allowed_values {
            if !allowed.contains(value) {
                return Err(anyhow!("Parameter '{}' has invalid value. Allowed values: {:?}", name, allowed));
            }
        }

        Ok(())
    }

    fn create_openai_config(name: &str) -> EngineConfig {
        let mut parameters = HashMap::new();
        parameters.insert("model".to_string(), Value::String("gpt-3.5-turbo".to_string()));
        parameters.insert("temperature".to_string(), Value::Number(serde_json::Number::from_f64(0.7).unwrap()));
        parameters.insert("max_tokens".to_string(), Value::Number(serde_json::Number::from(4096)));

        EngineConfig {
            name: name.to_string(),
            engine: "openai".to_string(),
            connection: ConnectionConfig {
                protocol: "https".to_string(),
                hostname: "api.openai.com".to_string(),
                port: 443,
                request_path: "/v1/chat/completions".to_string(),
            },
            parameters,
            session_id: None,
            neo4j: None,
            spinner: None,
        }
    }

    fn create_anthropic_config(name: &str) -> EngineConfig {
        let mut parameters = HashMap::new();
        parameters.insert("model".to_string(), Value::String("claude-3-sonnet".to_string()));
        parameters.insert("max_tokens".to_string(), Value::Number(serde_json::Number::from(4096)));

        EngineConfig {
            name: name.to_string(),
            engine: "anthropic".to_string(),
            connection: ConnectionConfig {
                protocol: "https".to_string(),
                hostname: "api.anthropic.com".to_string(),
                port: 443,
                request_path: "/v1/messages".to_string(),
            },
            parameters,
            session_id: None,
            neo4j: None,
            spinner: None,
        }
    }

    fn create_gemini_config(name: &str) -> EngineConfig {
        let mut parameters = HashMap::new();
        parameters.insert("model".to_string(), Value::String("gemini-1.5-flash".to_string()));

        EngineConfig {
            name: name.to_string(),
            engine: "google_gemini".to_string(),
            connection: ConnectionConfig {
                protocol: "https".to_string(),
                hostname: "generativelanguage.googleapis.com".to_string(),
                port: 443,
                request_path: "/v1beta/models/gemini-1.5-flash:generateContent".to_string(),
            },
            parameters,
            session_id: None,
            neo4j: None,
            spinner: None,
        }
    }

    fn create_generic_config(engine_type: &str, name: &str) -> EngineConfig {
        EngineConfig {
            name: name.to_string(),
            engine: engine_type.to_string(),
            connection: ConnectionConfig {
                protocol: "https".to_string(),
                hostname: "api.example.com".to_string(),
                port: 443,
                request_path: "/v1/chat/completions".to_string(),
            },
            parameters: HashMap::new(),
            session_id: None,
            neo4j: None,
            spinner: None,
        }
    }

    fn create_validation_rules(engine_type: &str) -> ValidationRules {
        match engine_type {
            "openai" => ValidationRules {
                required_parameters: vec!["model".to_string()],
                parameter_types: {
                    let mut types = HashMap::new();
                    types.insert("model".to_string(), ParameterType::String);
                    types.insert("temperature".to_string(), ParameterType::Number);
                    types.insert("max_tokens".to_string(), ParameterType::Number);
                    types.insert("bearer_token".to_string(), ParameterType::SecretString);
                    types
                },
                parameter_constraints: {
                    let mut constraints = HashMap::new();
                    constraints.insert("temperature".to_string(), ParameterConstraints {
                        min_value: Some(0.0),
                        max_value: Some(2.0),
                        min_length: None,
                        max_length: None,
                        allowed_values: None,
                        pattern: None,
                    });
                    constraints.insert("max_tokens".to_string(), ParameterConstraints {
                        min_value: Some(1.0),
                        max_value: Some(32768.0),
                        min_length: None,
                        max_length: None,
                        allowed_values: None,
                        pattern: None,
                    });
                    constraints
                },
                connection_timeout: Some(30),
                request_timeout: Some(120),
            },
            _ => ValidationRules {
                required_parameters: vec![],
                parameter_types: HashMap::new(),
                parameter_constraints: HashMap::new(),
                connection_timeout: Some(30),
                request_timeout: Some(60),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_config_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ConfigManager::new(temp_dir.path().to_path_buf());
        
        assert_eq!(manager.current_environment, "development");
    }

    #[tokio::test]
    async fn test_default_config_creation() {
        let config = ConfigManager::create_default_config("openai", "test-openai");
        
        assert_eq!(config.base.engine, "openai");
        assert_eq!(config.base.name, "test-openai");
        assert!(config.validation.required_parameters.contains(&"model".to_string()));
    }

    #[tokio::test]
    async fn test_config_validation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ConfigManager::new(temp_dir.path().to_path_buf());
        
        let config = ConfigManager::create_default_config("openai", "test");
        let result = manager.validate_config(&config).await;
        
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_parameter_validation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ConfigManager::new(temp_dir.path().to_path_buf());
        
        let mut config = ConfigManager::create_default_config("openai", "test");
        
        // Test invalid temperature
        config.base.parameters.insert("temperature".to_string(), Value::Number(serde_json::Number::from_f64(3.0).unwrap()));
        
        let result = manager.validate_config(&config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("above maximum"));
    }
}
