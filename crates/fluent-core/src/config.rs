use crate::neo4j_client::VoyageAIConfig;
use crate::spinner_configuration::SpinnerConfig;
use anyhow::{anyhow, Result};
use log::{debug, info};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::process::Command;
use std::sync::Arc;
use std::{env, fs};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EngineConfig {
    pub name: String,
    pub engine: String,
    pub connection: ConnectionConfig,
    pub parameters: HashMap<String, serde_json::Value>,
    pub session_id: Option<String>, // New field for sessionID
    pub neo4j: Option<Neo4jConfig>,
    pub spinner: Option<SpinnerConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Neo4jConfig {
    pub uri: String,
    pub user: String,
    pub password: String,
    pub database: String,
    pub voyage_ai: Option<VoyageAIConfig>,
    pub query_llm: Option<String>,
    pub parameters: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ConnectionConfig {
    pub protocol: String,
    pub hostname: String,
    pub port: u16,
    pub request_path: String,
}

#[derive(Clone)]
pub struct Config {
    pub engines: Vec<EngineConfig>,
    _env_guard: Arc<AmberEnvVarGuard>, // This keeps the guard alive
}

impl Config {
    pub fn new(engines: Vec<EngineConfig>, env_guard: AmberEnvVarGuard) -> Self {
        Config {
            engines,
            _env_guard: Arc::new(env_guard),
        }
    }
}
pub fn load_config(
    config_path: &str,
    engine_name: &str,
    overrides: &HashMap<String, String>,
) -> Result<Config> {
    let config_content = fs::read_to_string(config_path)?;
    let mut config: Value = serde_json::from_str(&config_content)?;

    debug!("Loading config for engine: {}", engine_name);

    let mut env_var_guard = AmberEnvVarGuard::new();

    // Find the specific engine configuration
    let engine_config = config["engines"]
        .as_array_mut()
        .ok_or_else(|| anyhow!("No engines found in configuration"))?
        .iter_mut()
        .find(|e| e["name"].as_str() == Some(engine_name))
        .ok_or_else(|| anyhow!("Engine '{}' not found in configuration", engine_name))?;

    // Decrypt Amber keys only for the specified engine
    env_var_guard.decrypt_amber_keys_in_value(engine_config)?;

    // Apply overrides to the specified engine
    if let Some(parameters) = engine_config["parameters"].as_object_mut() {
        for (key, value) in overrides {
            // Parse the override value to the correct type
            let parsed_value: Value = match parameters[key] {
                Value::Number(_) => value
                    .parse::<f64>()
                    .map(Value::from)
                    .unwrap_or(Value::String(value.clone())),
                Value::Bool(_) => value
                    .parse::<bool>()
                    .map(Value::from)
                    .unwrap_or(Value::String(value.clone())),
                _ => Value::String(value.clone()),
            };
            parameters.insert(key.clone(), parsed_value);
        }
    }

    debug!("Loaded and processed config for engine: {}", engine_name);

    let engine_config: EngineConfig = serde_json::from_value(engine_config.clone())?;

    Ok(Config::new(vec![engine_config], env_var_guard))
}

pub fn parse_key_value_pair(pair: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = pair.splitn(2, '=').collect();
    if parts.len() == 2 {
        Some((parts[0].to_string(), parts[1].to_string()))
    } else {
        None
    }
}

pub fn apply_overrides(config: &mut EngineConfig, overrides: &[(String, String)]) -> Result<()> {
    for (key, value) in overrides {
        let value = serde_json::from_str(value).unwrap_or(serde_json::Value::String(value.clone()));
        config.parameters.insert(key.clone(), value);
    }
    Ok(())
}

#[derive(Default)]
pub struct AmberEnvVarGuard {
    keys: Vec<String>,
}

impl AmberEnvVarGuard {
    pub fn new() -> Self {
        AmberEnvVarGuard::default()
    }

    fn decrypt_amber_keys_in_value(&mut self, value: &mut Value) -> Result<()> {
        match value {
            Value::String(s) if s.starts_with("AMBER_") => {
                let decrypted = self.get_amber_value(s)?;
                self.set_env_var_from_amber(s, &decrypted)?;
                *s = decrypted;
                Ok(())
            }
            Value::Object(map) => {
                for (_, v) in map.iter_mut() {
                    self.decrypt_amber_keys_in_value(v)?;
                }
                Ok(())
            }
            Value::Array(arr) => {
                for item in arr.iter_mut() {
                    self.decrypt_amber_keys_in_value(item)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn set_env_var_from_amber(&mut self, key: &str, value: &str) -> Result<()> {
        std::env::set_var(key, value);
        debug!("Set environment variable {} with decrypted value", key);
        self.keys.push(key.to_owned());
        Ok(())
    }

    fn get_amber_value(&self, key: &str) -> Result<String> {
        let output = Command::new("amber").arg("print").output()?;

        if !output.status.success() {
            return Err(anyhow!("Failed to run amber print command"));
        }

        let stdout = String::from_utf8(output.stdout)?;
        //debug!("Amber print output: {}", stdout);
        for line in stdout.lines() {
            if line.contains(key) {
                let parts: Vec<&str> = line.splitn(2, '=').collect();
                if parts.len() == 2 {
                    let value = parts[1].trim().trim_matches('"');
                    return Ok(value.to_string());
                }
            }
        }

        Err(anyhow!("Amber key not found: {}", key))
    }
}

impl Drop for AmberEnvVarGuard {
    fn drop(&mut self) {
        for key in &self.keys {
            std::env::remove_var(key);
            info!("Environment variable {} has been unset.", key);
        }
    }
}

// Helper function to replace config strings starting with "AMBER_" with their env values
pub fn replace_with_env_var(value: &mut Value) {
    match value {
        Value::String(s) if s.starts_with("AMBER_") => {
            let env_key = &s[6..]; // Skip the "AMBER_" prefix to fetch the correct env var
            debug!("Attempting to replace: {}", s);
            debug!("Looking up environment variable: {}", env_key);
            match env::var(env_key) {
                Ok(env_value) => {
                    debug!("Environment value found for: {}", env_key);
                    *s = env_value;
                }
                Err(e) => {
                    debug!("Failed to find environment variable '{}': {}", env_key, e);
                }
            }
        }
        Value::Object(map) => {
            for (_, v) in map.iter_mut() {
                replace_with_env_var(v);
            }
        }
        Value::Array(arr) => {
            for item in arr.iter_mut() {
                replace_with_env_var(item);
            }
        }
        _ => {}
    }
}
