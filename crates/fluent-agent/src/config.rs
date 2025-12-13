use anyhow::{anyhow, Result};
use fluent_core::config::load_engine_config;
use fluent_core::traits::Engine;
use fluent_engines::create_engine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tracing::warn;
// use std::time::Duration;

use crate::autonomy::AutonomySupervisorConfig;
use crate::performance::PerformanceConfig;
use crate::state_manager::StateManagerConfig;

/// Rate limiting configuration for API calls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Whether rate limiting is enabled
    pub enabled: bool,
    /// Maximum requests per second for reasoning engine
    pub reasoning_rps: f64,
    /// Maximum requests per second for action engine
    pub action_rps: f64,
    /// Maximum requests per second for reflection engine
    pub reflection_rps: f64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            reasoning_rps: 5.0,  // 5 requests per second
            action_rps: 10.0,    // 10 requests per second
            reflection_rps: 3.0, // 3 requests per second
        }
    }
}

impl RateLimitConfig {
    /// Create from environment variables
    pub fn from_environment() -> Self {
        let enabled = std::env::var("FLUENT_RATE_LIMIT_ENABLED")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(true);

        let reasoning_rps = std::env::var("FLUENT_REASONING_RPS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5.0);

        let action_rps = std::env::var("FLUENT_ACTION_RPS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10.0);

        let reflection_rps = std::env::var("FLUENT_REFLECTION_RPS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3.0);

        Self {
            enabled,
            reasoning_rps,
            action_rps,
            reflection_rps,
        }
    }
}

/// Configuration for the agentic framework that integrates with fluent_cli's existing patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub agent: AgentEngineConfig,
}

/// Engine configuration for different agent components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEngineConfig {
    pub reasoning_engine: String,
    pub action_engine: String,
    pub reflection_engine: String,
    pub memory_database: String,
    #[serde(default = "default_memory_enabled")]
    pub memory_enabled: bool,
    pub tools: ToolConfig,
    pub config_path: Option<String>,
    pub max_iterations: Option<u32>,
    pub timeout_seconds: Option<u64>,
    pub supervisor: Option<AutonomySupervisorConfig>,
    pub performance: Option<PerformanceConfig>,
    pub state_management: Option<StateManagerConfig>,
    pub rate_limit: Option<RateLimitConfig>,
}

/// Tool configuration for the agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    pub file_operations: bool,
    pub shell_commands: bool,
    pub rust_compiler: bool,
    pub git_operations: bool,
    #[serde(default = "default_web_browsing")]
    pub web_browsing: bool,
    pub allowed_paths: Option<Vec<String>>,
    pub allowed_commands: Option<Vec<String>>,
}

fn default_web_browsing() -> bool {
    true
}

fn default_memory_enabled() -> bool {
    true
}

/// Runtime configuration with loaded engines and credentials
#[derive(Clone)]
pub struct AgentRuntimeConfig {
    pub reasoning_engine: Arc<dyn Engine>,
    pub action_engine: Arc<dyn Engine>,
    pub reflection_engine: Arc<dyn Engine>,
    pub config: AgentEngineConfig,
    pub credentials: HashMap<String, String>,
    pub supervisor: Option<AutonomySupervisorConfig>,
    pub performance: PerformanceConfig,
    pub state_overrides: Option<StateManagerConfig>,
    pub rate_limit: RateLimitConfig,
    pub reasoning_rate_limiter: Option<Arc<fluent_engines::RateLimiter>>,
    pub action_rate_limiter: Option<Arc<fluent_engines::RateLimiter>>,
    pub reflection_rate_limiter: Option<Arc<fluent_engines::RateLimiter>>,
}

impl AgentRuntimeConfig {
    /// Get the base engine for enhanced reasoning
    pub fn get_base_engine(&self) -> Option<Arc<dyn Engine>> {
        Some(self.reasoning_engine.clone())
    }

    /// Acquire a rate limit token for reasoning operations
    ///
    /// If rate limiting is disabled, returns immediately.
    /// Otherwise, waits until a token is available.
    pub async fn acquire_reasoning_rate_limit(&self) {
        if let Some(ref limiter) = self.reasoning_rate_limiter {
            limiter.acquire().await;
        }
    }

    /// Acquire a rate limit token for action operations
    pub async fn acquire_action_rate_limit(&self) {
        if let Some(ref limiter) = self.action_rate_limiter {
            limiter.acquire().await;
        }
    }

    /// Acquire a rate limit token for reflection operations
    pub async fn acquire_reflection_rate_limit(&self) {
        if let Some(ref limiter) = self.reflection_rate_limiter {
            limiter.acquire().await;
        }
    }

    /// Try to acquire a rate limit token without blocking
    ///
    /// Returns true if token was acquired, false if rate limited.
    pub async fn try_acquire_reasoning_rate_limit(&self) -> bool {
        if let Some(ref limiter) = self.reasoning_rate_limiter {
            limiter.try_acquire().await
        } else {
            true // No limiter = always allowed
        }
    }

    /// Try to acquire an action rate limit token without blocking
    pub async fn try_acquire_action_rate_limit(&self) -> bool {
        if let Some(ref limiter) = self.action_rate_limiter {
            limiter.try_acquire().await
        } else {
            true
        }
    }

    /// Try to acquire a reflection rate limit token without blocking
    pub async fn try_acquire_reflection_rate_limit(&self) -> bool {
        if let Some(ref limiter) = self.reflection_rate_limiter {
            limiter.try_acquire().await
        } else {
            true
        }
    }

    /// Get the current rate limit configuration
    pub fn rate_limit_config(&self) -> &RateLimitConfig {
        &self.rate_limit
    }

    /// Check if rate limiting is enabled
    pub fn is_rate_limiting_enabled(&self) -> bool {
        self.rate_limit.enabled
    }

    /// Get available reasoning tokens (for monitoring)
    pub async fn reasoning_tokens_available(&self) -> f64 {
        if let Some(ref limiter) = self.reasoning_rate_limiter {
            limiter.available_tokens().await
        } else {
            f64::INFINITY
        }
    }

    /// Get available action tokens (for monitoring)
    pub async fn action_tokens_available(&self) -> f64 {
        if let Some(ref limiter) = self.action_rate_limiter {
            limiter.available_tokens().await
        } else {
            f64::INFINITY
        }
    }

    /// Get available reflection tokens (for monitoring)
    pub async fn reflection_tokens_available(&self) -> f64 {
        if let Some(ref limiter) = self.reflection_rate_limiter {
            limiter.available_tokens().await
        } else {
            f64::INFINITY
        }
    }
}

impl AgentEngineConfig {
    /// Load agent configuration from file
    pub async fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = tokio::fs::read_to_string(path).await?;
        let config: AgentConfig = serde_json::from_str(&content)?;
        Ok(config.agent)
    }

    /// Create runtime configuration with real engines
    pub async fn create_runtime_config(
        &self,
        fluent_config_path: &str,
        credentials: HashMap<String, String>,
        model_override: Option<&str>,
    ) -> Result<AgentRuntimeConfig> {
        // Load the main fluent_cli configuration
        let fluent_config_content = tokio::fs::read_to_string(fluent_config_path).await?;

        // Create reasoning engine
        let reasoning_engine = self
            .create_engine(
                &fluent_config_content,
                &self.reasoning_engine,
                &credentials,
                model_override,
            )
            .await?;

        // Create action engine (can be the same as reasoning)
        let action_engine = if self.action_engine == self.reasoning_engine {
            // Create a new instance of the same engine
            self.create_engine(
                &fluent_config_content,
                &self.action_engine,
                &credentials,
                model_override,
            )
            .await?
        } else {
            self.create_engine(
                &fluent_config_content,
                &self.action_engine,
                &credentials,
                model_override,
            )
            .await?
        };

        // Create reflection engine (can be the same as reasoning)
        let reflection_engine = self
            .create_engine(
                &fluent_config_content,
                &self.reflection_engine,
                &credentials,
                model_override,
            )
            .await?;

        // Initialize rate limiters based on config
        let rate_limit_config = self
            .rate_limit
            .clone()
            .unwrap_or_else(RateLimitConfig::from_environment);

        let (reasoning_rate_limiter, action_rate_limiter, reflection_rate_limiter) =
            if rate_limit_config.enabled {
                (
                    Some(Arc::new(fluent_engines::RateLimiter::new(
                        rate_limit_config.reasoning_rps,
                    ))),
                    Some(Arc::new(fluent_engines::RateLimiter::new(
                        rate_limit_config.action_rps,
                    ))),
                    Some(Arc::new(fluent_engines::RateLimiter::new(
                        rate_limit_config.reflection_rps,
                    ))),
                )
            } else {
                (None, None, None)
            };

        Ok(AgentRuntimeConfig {
            reasoning_engine: Arc::from(reasoning_engine),
            action_engine: Arc::from(action_engine),
            reflection_engine: Arc::from(reflection_engine),
            config: self.clone(),
            credentials,
            supervisor: self.supervisor.clone(),
            performance: self.performance.clone().unwrap_or_default(),
            state_overrides: self.state_management.clone(),
            rate_limit: rate_limit_config,
            reasoning_rate_limiter,
            action_rate_limiter,
            reflection_rate_limiter,
        })
    }

    /// Resolve the configured SQLite memory database to a filesystem path.
    ///
    /// Supported forms:
    /// - `sqlite://global` (default)
    /// - `sqlite://:memory:`
    /// - `sqlite:///absolute/path/to/db`
    /// - `sqlite://./relative/path/to/db`
    pub fn resolve_memory_db_path(&self) -> std::path::PathBuf {
        let url = self.memory_database.trim();
        let Some(rest) = url.strip_prefix("sqlite://") else {
            return crate::paths::global_agent_memory_db_path();
        };

        if rest.is_empty() || rest == "global" {
            return crate::paths::global_agent_memory_db_path();
        }
        if rest == ":memory:" {
            return std::path::PathBuf::from(":memory:");
        }

        // `sqlite:///abs/path` yields rest like "/abs/path" (keep it absolute)
        // `sqlite://./rel/path` yields rest like "./rel/path"
        std::path::PathBuf::from(rest)
    }

    pub fn supervisor_config(&self) -> AutonomySupervisorConfig {
        self.supervisor.clone().unwrap_or_default()
    }

    pub fn performance_config(&self) -> PerformanceConfig {
        self.performance.clone().unwrap_or_default()
    }

    pub fn state_manager_overrides(&self) -> Option<StateManagerConfig> {
        self.state_management.clone()
    }

    /// Create a specific engine using fluent_cli's configuration system with fallback
    async fn create_engine(
        &self,
        config_content: &str,
        engine_name: &str,
        credentials: &HashMap<String, String>,
        model_override: Option<&str>,
    ) -> Result<Box<dyn Engine>> {
        // First try to load from the provided config file
        match load_engine_config(
            config_content,
            engine_name,
            &HashMap::new(), // No overrides for now
            credentials,
        ) {
            Ok(engine_config) => {
                // Try to create the engine with the loaded config
                match create_engine(&engine_config).await {
                    Ok(engine) => Ok(engine),
                    Err(e) => {
                        warn!(
                            "Failed to create engine '{}' with config: {}",
                            engine_name, e
                        );
                        self.create_default_engine(engine_name, credentials, model_override)
                            .await
                    }
                }
            }
            Err(e) => {
                warn!("Engine '{}' not found in config: {}", engine_name, e);
                self.create_default_engine(engine_name, credentials, model_override)
                    .await
            }
        }
    }

    /// Create a default engine configuration for common engines
    async fn create_default_engine(
        &self,
        engine_name: &str,
        credentials: &HashMap<String, String>,
        model_override: Option<&str>,
    ) -> Result<Box<dyn Engine>> {
        use fluent_core::config::{ConnectionConfig, EngineConfig};
        use serde_json::Value;
        use std::collections::HashMap as StdHashMap;

        let (engine_type, api_key_name, hostname, request_path, mut model) = match engine_name {
            "openai" | "gpt-4o" | "gpt-4" => (
                "openai",
                "OPENAI_API_KEY",
                "api.openai.com",
                "/v1/chat/completions",
                "gpt-4",
            ),
            "anthropic" | "claude" | "sonnet3.5" => (
                "anthropic",
                "ANTHROPIC_API_KEY",
                "api.anthropic.com",
                "/v1/messages",
                "claude-sonnet-4-20250514",
            ),
            "google_gemini" | "gemini" | "gemini-flash" => (
                "google_gemini",
                "GOOGLE_API_KEY",
                "generativelanguage.googleapis.com",
                "/v1beta/models",
                "gemini-1.5-flash",
            ),
            "groq" | "groq_lpu" => (
                "groq_lpu",
                "GROQ_API_KEY",
                "api.groq.com",
                "/openai/v1/chat/completions",
                "llama-3.1-70b-versatile",
            ),
            "perplexity" => (
                "perplexity",
                "PERPLEXITY_API_KEY",
                "api.perplexity.ai",
                "/chat/completions",
                "llama-3.1-sonar-large-128k-online",
            ),
            _ => return Err(anyhow!("Unsupported engine type: {}", engine_name)),
        };

        // Apply model override if provided
        if let Some(override_model) = model_override {
            model = override_model;
        }

        // Check if we have the required API key
        let api_key = credentials.get(api_key_name)
            .or_else(|| credentials.get(&api_key_name.replace("_API_KEY", "")))
            .ok_or_else(|| anyhow!("Missing API key '{}' for engine '{}'. Please set the environment variable or add it to your credentials.", api_key_name, engine_name))?;

        let mut parameters = StdHashMap::new();
        // Fluent engines expect 'bearer_token' and 'modelName'
        parameters.insert("bearer_token".to_string(), Value::String(api_key.clone()));
        parameters.insert("modelName".to_string(), Value::String(model.to_string()));
        if let Some(temp_number) = serde_json::Number::from_f64(0.1) {
            parameters.insert("temperature".to_string(), Value::Number(temp_number));
        }

        if engine_type != "google_gemini" {
            parameters.insert(
                "max_tokens".to_string(),
                Value::Number(serde_json::Number::from(4000)),
            );
        }

        let engine_config = EngineConfig {
            name: engine_name.to_string(),
            engine: engine_type.to_string(),
            connection: ConnectionConfig {
                protocol: "https".to_string(),
                hostname: hostname.to_string(),
                port: 443,
                request_path: request_path.to_string(),
            },
            parameters,
            session_id: None,
            neo4j: None,
            spinner: None,
        };

        println!(
            "🔧 Creating default {} engine with model {}",
            engine_type, model
        );
        create_engine(&engine_config)
            .await
            .map_err(|e| anyhow!("Failed to create default engine '{}': {}", engine_name, e))
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.reasoning_engine.is_empty() {
            return Err(anyhow!("Reasoning engine name cannot be empty"));
        }

        if self.action_engine.is_empty() {
            return Err(anyhow!("Action engine name cannot be empty"));
        }

        if self.reflection_engine.is_empty() {
            return Err(anyhow!("Reflection engine name cannot be empty"));
        }

        if self.memory_database.is_empty() {
            return Err(anyhow!("Memory database URL cannot be empty"));
        }

        // Validate memory database URL format
        if !self.memory_database.starts_with("sqlite://") {
            return Err(anyhow!("Only SQLite databases are currently supported"));
        }

        // Validate tool configuration
        if let Some(ref paths) = self.tools.allowed_paths {
            for path in paths {
                if path.is_empty() {
                    return Err(anyhow!("Allowed paths cannot contain empty strings"));
                }
            }
        }

        if let Some(ref commands) = self.tools.allowed_commands {
            for command in commands {
                if command.is_empty() {
                    return Err(anyhow!("Allowed commands cannot contain empty strings"));
                }
            }
        }

        Ok(())
    }

    /// Get default configuration
    pub fn default_config() -> Self {
        Self {
            reasoning_engine: "sonnet3.5".to_string(),
            action_engine: "gpt-4o".to_string(),
            reflection_engine: "gemini-flash".to_string(),
            memory_database: "sqlite://global".to_string(),
            memory_enabled: true,
            tools: ToolConfig {
                file_operations: true,
                shell_commands: false, // Disabled by default for security
                rust_compiler: true,
                git_operations: false, // Disabled by default for security
                web_browsing: true,
                allowed_paths: Some(vec![
                    "./".to_string(),
                    "./src".to_string(),
                    "./examples".to_string(),
                    "./tests".to_string(),
                ]),
                allowed_commands: Some(vec![
                    "cargo build".to_string(),
                    "cargo test".to_string(),
                    "cargo check".to_string(),
                    "cargo clippy".to_string(),
                ]),
            },
            config_path: Some("./config.json".to_string()),
            max_iterations: Some(50),
            timeout_seconds: Some(1800), // 30 minutes
            supervisor: None,
            performance: None,
            state_management: None,
            rate_limit: Some(RateLimitConfig::default()),
        }
    }

    /// Create agent configuration template file
    pub async fn create_template_file<P: AsRef<Path>>(path: P) -> Result<()> {
        let config = AgentConfig {
            agent: Self::default_config(),
        };

        let json = serde_json::to_string_pretty(&config)?;
        tokio::fs::write(path, json).await?;
        Ok(())
    }
}

impl Default for ToolConfig {
    fn default() -> Self {
        Self {
            file_operations: true,
            shell_commands: false,
            rust_compiler: true,
            git_operations: false,
            web_browsing: true,
            allowed_paths: Some(vec![
                "./".to_string(),
                "./src".to_string(),
                "./examples".to_string(),
                "./tests".to_string(),
            ]),
            allowed_commands: Some(vec![
                "cargo build".to_string(),
                "cargo test".to_string(),
                "cargo check".to_string(),
                "cargo clippy".to_string(),
            ]),
        }
    }
}

/// Utility functions for credential management
pub mod credentials {
    use super::*;
    use std::env;

    /// Load credentials using fluent_cli's comprehensive credential system
    /// This includes environment variables, amber store, and CREDENTIAL_ prefixed variables
    pub fn load_from_environment() -> HashMap<String, String> {
        let mut credentials = HashMap::new();

        // Load direct environment variables (fluent_cli pattern)
        let credential_keys = [
            "OPENAI_API_KEY",
            "ANTHROPIC_API_KEY",
            "GOOGLE_API_KEY",
            "GEMINI_API_KEY",
            "GROQ_API_KEY",
            "PERPLEXITY_API_KEY",
            "COHERE_API_KEY",
            "MISTRAL_API_KEY",
        ];

        for key in &credential_keys {
            if let Ok(value) = env::var(key) {
                credentials.insert(key.to_string(), value);
            }
        }

        // Load CREDENTIAL_ prefixed variables (fluent_cli pattern)
        for (key, value) in env::vars() {
            if let Some(credential_key) = key.strip_prefix("CREDENTIAL_") {
                credentials.insert(credential_key.to_string(), value);
            }
        }

        // Try to load from amber store if available (with security validation)
        if let Ok(amber_path) = which::which("amber") {
            if let Ok(output) = std::process::Command::new(amber_path)
                .arg("print")
                .env_clear() // Clear environment for security
                .output()
            {
                if output.status.success() {
                    if let Ok(stdout) = String::from_utf8(output.stdout) {
                        for line in stdout.lines() {
                            if let Some((key, value)) = parse_amber_line(line) {
                                // Validate key format to prevent injection
                                if key.chars().all(|c| c.is_alphanumeric() || c == '_')
                                    && credential_keys.iter().any(|&k| k == key)
                                {
                                    credentials.insert(key, value);
                                }
                            }
                        }
                    }
                }
            }
        }

        credentials
    }

    /// Parse a line from amber print output
    fn parse_amber_line(line: &str) -> Option<(String, String)> {
        if let Some((key, value)) = fluent_core::config::parse_key_value_pair(line) {
            let key = key.trim().to_string();
            let value = value.trim().trim_matches('"').to_string();
            Some((key, value))
        } else {
            None
        }
    }

    /// Validate that required credentials are available
    pub fn validate_credentials(
        credentials: &HashMap<String, String>,
        required_engines: &[String],
    ) -> Result<()> {
        // Map engine names to required credential keys
        let engine_credentials = [
            ("openai", "OPENAI_API_KEY"),
            ("gpt-4o", "OPENAI_API_KEY"),
            ("gpt-4", "OPENAI_API_KEY"),
            ("sonnet3.5", "ANTHROPIC_API_KEY"),
            ("claude", "ANTHROPIC_API_KEY"),
            ("anthropic", "ANTHROPIC_API_KEY"),
            ("gemini", "GOOGLE_API_KEY"),
            ("gemini-flash", "GOOGLE_API_KEY"),
            ("google", "GOOGLE_API_KEY"),
        ];

        for engine in required_engines {
            let engine_lower = engine.to_lowercase();

            // Find the required credential for this engine
            if let Some((_, credential_key)) = engine_credentials
                .iter()
                .find(|(engine_name, _)| engine_lower.contains(engine_name))
            {
                if !credentials.contains_key(*credential_key) {
                    return Err(anyhow!(
                        "Missing required credential '{}' for engine '{}'. Please set the environment variable.",
                        credential_key,
                        engine
                    ));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config_validation() {
        let config = AgentEngineConfig::default_config();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_config_validation() {
        let mut config = AgentEngineConfig::default_config();
        config.reasoning_engine = "".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_invalid_database_url() {
        let mut config = AgentEngineConfig::default_config();
        config.memory_database = "invalid://url".to_string();
        assert!(config.validate().is_err());
    }

    #[tokio::test]
    async fn test_config_file_creation() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        AgentEngineConfig::create_template_file(path).await.unwrap();

        // Verify file was created and can be loaded
        let loaded_config = AgentEngineConfig::load_from_file(path).await.unwrap();
        assert!(loaded_config.validate().is_ok());
    }

    #[test]
    fn test_credential_validation() {
        let mut credentials = HashMap::new();
        credentials.insert("OPENAI_API_KEY".to_string(), "test-key".to_string());

        let engines = vec!["gpt-4o".to_string()];
        assert!(credentials::validate_credentials(&credentials, &engines).is_ok());

        let engines = vec!["sonnet3.5".to_string()];
        assert!(credentials::validate_credentials(&credentials, &engines).is_err());
    }

    #[test]
    fn test_rate_limit_config_default() {
        let config = RateLimitConfig::default();
        assert!(config.enabled);
        assert_eq!(config.reasoning_rps, 5.0);
        assert_eq!(config.action_rps, 10.0);
        assert_eq!(config.reflection_rps, 3.0);
    }

    #[test]
    fn test_rate_limit_config_from_env() {
        // Test that environment variables are read correctly
        std::env::set_var("FLUENT_RATE_LIMIT_ENABLED", "false");
        std::env::set_var("FLUENT_REASONING_RPS", "2.5");

        let config = RateLimitConfig::from_environment();
        assert!(!config.enabled);
        assert_eq!(config.reasoning_rps, 2.5);

        // Clean up
        std::env::remove_var("FLUENT_RATE_LIMIT_ENABLED");
        std::env::remove_var("FLUENT_REASONING_RPS");
    }

    #[test]
    fn test_default_config_includes_rate_limit() {
        let config = AgentEngineConfig::default_config();
        assert!(config.rate_limit.is_some());
        let rate_limit = config.rate_limit.unwrap();
        assert!(rate_limit.enabled);
    }
}
