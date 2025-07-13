use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;
use std::sync::OnceLock;

/// Centralized configuration for the entire fluent_cli system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FluentConfig {
    /// Application-wide settings
    pub app: AppConfig,
    
    /// Pipeline-specific configuration
    pub pipeline: PipelineConfig,
    
    /// Engine default configurations
    pub engines: EngineDefaults,
    
    /// Directory and path configurations
    pub paths: PathConfig,
    
    /// Network and timeout configurations
    pub network: NetworkConfig,
    
    /// Security and validation settings
    pub security: SecurityConfig,
}

/// Application-wide configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub name: String,
    pub version: String,
    pub log_level: String,
    pub max_concurrent_operations: usize,
    pub default_session_id: String,
}

/// Pipeline execution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub default_timeout_seconds: u64,
    pub max_parallel_steps: usize,
    pub retry_attempts: u32,
    pub retry_base_delay_ms: u64,
    pub retry_max_delay_ms: u64,
    pub retry_backoff_multiplier: f64,
}

/// Default engine configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineDefaults {
    pub openai: OpenAIDefaults,
    pub anthropic: AnthropicDefaults,
    pub google_gemini: GoogleGeminiDefaults,
    pub timeout_ms: u64,
    pub max_tokens: i32,
    pub temperature: f64,
}

/// OpenAI engine defaults
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIDefaults {
    pub hostname: String,
    pub port: u16,
    pub request_path: String,
    pub model: String,
    pub max_tokens: i32,
    pub temperature: f64,
    pub top_p: f64,
    pub frequency_penalty: f64,
    pub presence_penalty: f64,
}

/// Anthropic engine defaults
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicDefaults {
    pub hostname: String,
    pub port: u16,
    pub request_path: String,
    pub model: String,
    pub max_tokens: i32,
    pub temperature: f64,
}

/// Google Gemini engine defaults
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleGeminiDefaults {
    pub hostname: String,
    pub port: u16,
    pub request_path_template: String,
    pub model: String,
    pub temperature: f64,
}

/// Path and directory configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathConfig {
    pub pipeline_directory: PathBuf,
    pub pipeline_state_directory: PathBuf,
    pub pipeline_logs_directory: PathBuf,
    pub config_directory: PathBuf,
    pub cache_directory: PathBuf,
    pub plugin_directory: PathBuf,
    pub audit_log_path: PathBuf,
}

/// Network and timeout configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub default_timeout_ms: u64,
    pub connection_timeout_ms: u64,
    pub read_timeout_ms: u64,
    pub max_retries: u32,
    pub retry_delay_ms: u64,
    pub max_concurrent_requests: usize,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub signature_verification_enabled: bool,
    pub max_plugins: usize,
    pub audit_logging_enabled: bool,
    pub credential_validation_enabled: bool,
    pub allowed_file_extensions: Vec<String>,
    pub max_file_size_mb: u64,
}

impl Default for FluentConfig {
    fn default() -> Self {
        Self {
            app: AppConfig {
                name: "fluent_cli".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                log_level: "info".to_string(),
                max_concurrent_operations: 10,
                default_session_id: "DEFAULT_SESSION_ID".to_string(),
            },
            pipeline: PipelineConfig {
                default_timeout_seconds: 300,
                max_parallel_steps: 2,
                retry_attempts: 3,
                retry_base_delay_ms: 1000,
                retry_max_delay_ms: 10000,
                retry_backoff_multiplier: 2.0,
            },
            engines: EngineDefaults {
                openai: OpenAIDefaults {
                    hostname: "api.openai.com".to_string(),
                    port: 443,
                    request_path: "/v1/chat/completions".to_string(),
                    model: "gpt-4o-mini".to_string(),
                    max_tokens: 4096,
                    temperature: 0.7,
                    top_p: 1.0,
                    frequency_penalty: 0.0,
                    presence_penalty: 0.0,
                },
                anthropic: AnthropicDefaults {
                    hostname: "api.anthropic.com".to_string(),
                    port: 443,
                    request_path: "/v1/messages".to_string(),
                    model: "claude-3-5-sonnet-20241022".to_string(),
                    max_tokens: 2000,
                    temperature: 0.7,
                },
                google_gemini: GoogleGeminiDefaults {
                    hostname: "generativelanguage.googleapis.com".to_string(),
                    port: 443,
                    request_path_template: "/v1beta/models/{model}:generateContent".to_string(),
                    model: "gemini-1.5-flash".to_string(),
                    temperature: 0.7,
                },
                timeout_ms: 30000,
                max_tokens: 4096,
                temperature: 0.7,
            },
            paths: PathConfig {
                pipeline_directory: PathBuf::from("./pipelines"),
                pipeline_state_directory: PathBuf::from("./pipeline_states"),
                pipeline_logs_directory: PathBuf::from("./pipeline_logs"),
                config_directory: PathBuf::from("./config"),
                cache_directory: PathBuf::from("./cache"),
                plugin_directory: PathBuf::from("./plugins"),
                audit_log_path: PathBuf::from("./audit.log"),
            },
            network: NetworkConfig {
                default_timeout_ms: 30000,
                connection_timeout_ms: 10000,
                read_timeout_ms: 30000,
                max_retries: 3,
                retry_delay_ms: 1000,
                max_concurrent_requests: 10,
            },
            security: SecurityConfig {
                signature_verification_enabled: true,
                max_plugins: 50,
                audit_logging_enabled: true,
                credential_validation_enabled: true,
                allowed_file_extensions: vec![
                    "json".to_string(),
                    "yaml".to_string(),
                    "yml".to_string(),
                    "toml".to_string(),
                    "txt".to_string(),
                ],
                max_file_size_mb: 100,
            },
        }
    }
}

/// Global configuration instance
static GLOBAL_CONFIG: OnceLock<FluentConfig> = OnceLock::new();

/// Configuration manager for loading and accessing centralized configuration
pub struct ConfigManager;

impl ConfigManager {
    /// Initialize the global configuration from file or environment
    pub fn initialize() -> Result<()> {
        let config = Self::load_config()?;
        GLOBAL_CONFIG.set(config)
            .map_err(|_| anyhow!("Global configuration already initialized"))?;
        Ok(())
    }

    /// Get the global configuration instance
    pub fn get() -> &'static FluentConfig {
        GLOBAL_CONFIG.get_or_init(|| FluentConfig::default())
    }

    /// Load configuration from file with environment variable overrides
    fn load_config() -> Result<FluentConfig> {
        // Try to load from config file first
        let config_path = Self::get_config_path();
        
        let mut config = if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            if config_path.extension().and_then(|s| s.to_str()) == Some("toml") {
                toml::from_str(&content)?
            } else {
                serde_json::from_str(&content)?
            }
        } else {
            FluentConfig::default()
        };

        // Apply environment variable overrides
        Self::apply_env_overrides(&mut config)?;

        Ok(config)
    }

    /// Get the configuration file path from environment or default
    fn get_config_path() -> PathBuf {
        env::var("FLUENT_CONFIG_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("./fluent_config.json"))
    }

    /// Apply environment variable overrides to configuration
    fn apply_env_overrides(config: &mut FluentConfig) -> Result<()> {
        // Pipeline configuration overrides
        if let Ok(timeout) = env::var("FLUENT_PIPELINE_TIMEOUT") {
            config.pipeline.default_timeout_seconds = timeout.parse()?;
        }
        
        if let Ok(max_parallel) = env::var("FLUENT_PIPELINE_MAX_PARALLEL") {
            config.pipeline.max_parallel_steps = max_parallel.parse()?;
        }

        // Path overrides
        if let Ok(pipeline_dir) = env::var("FLUENT_PIPELINE_DIR") {
            config.paths.pipeline_directory = PathBuf::from(pipeline_dir);
        }
        
        if let Ok(state_dir) = env::var("FLUENT_PIPELINE_STATE_DIR") {
            config.paths.pipeline_state_directory = PathBuf::from(state_dir);
        }

        // Network configuration overrides
        if let Ok(timeout) = env::var("FLUENT_NETWORK_TIMEOUT") {
            config.network.default_timeout_ms = timeout.parse()?;
        }

        // Engine defaults overrides
        if let Ok(model) = env::var("FLUENT_OPENAI_DEFAULT_MODEL") {
            config.engines.openai.model = model;
        }
        
        if let Ok(temp) = env::var("FLUENT_DEFAULT_TEMPERATURE") {
            config.engines.temperature = temp.parse()?;
        }

        Ok(())
    }

    /// Save current configuration to file
    pub fn save_config(config: &FluentConfig) -> Result<()> {
        let config_path = Self::get_config_path();
        
        // Create config directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = if config_path.extension().and_then(|s| s.to_str()) == Some("toml") {
            toml::to_string_pretty(config)?
        } else {
            serde_json::to_string_pretty(config)?
        };

        std::fs::write(&config_path, content)?;
        Ok(())
    }

    /// Validate configuration values
    pub fn validate_config(config: &FluentConfig) -> Result<()> {
        // Validate timeout values
        if config.pipeline.default_timeout_seconds == 0 {
            return Err(anyhow!("Pipeline timeout must be greater than 0"));
        }

        if config.network.default_timeout_ms == 0 {
            return Err(anyhow!("Network timeout must be greater than 0"));
        }

        // Validate path configurations
        if config.paths.pipeline_directory.as_os_str().is_empty() {
            return Err(anyhow!("Pipeline directory cannot be empty"));
        }

        // Validate engine configurations
        if config.engines.openai.hostname.is_empty() {
            return Err(anyhow!("OpenAI hostname cannot be empty"));
        }

        if config.engines.openai.port == 0 {
            return Err(anyhow!("OpenAI port must be greater than 0"));
        }

        // Validate temperature ranges
        if !(0.0..=2.0).contains(&config.engines.temperature) {
            return Err(anyhow!("Temperature must be between 0.0 and 2.0"));
        }

        Ok(())
    }
}

/// Convenience functions for accessing common configuration values
impl FluentConfig {
    /// Get pipeline state directory with environment override
    pub fn get_pipeline_state_dir(&self) -> PathBuf {
        env::var("FLUENT_PIPELINE_STATE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| self.paths.pipeline_state_directory.clone())
    }

    /// Get pipeline directory with environment override
    pub fn get_pipeline_dir(&self) -> PathBuf {
        env::var("FLUENT_PIPELINE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| self.paths.pipeline_directory.clone())
    }

    /// Get default timeout with environment override
    pub fn get_default_timeout_ms(&self) -> u64 {
        env::var("FLUENT_NETWORK_TIMEOUT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(self.network.default_timeout_ms)
    }
}
