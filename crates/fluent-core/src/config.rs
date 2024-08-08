use crate::neo4j_client::VoyageAIConfig;
use crate::spinner_configuration::SpinnerConfig;
use anyhow::{anyhow, Context, Result};
use log::debug;
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
}

impl Config {
    pub fn new(engines: Vec<EngineConfig>) -> Self {
        Config { engines }
    }
}

pub trait VariableResolver {
    fn is_resolvable(&self, key: &str) -> bool;
    fn resolve(&self, key: &str) -> Result<String>;
}
pub struct AmberVarResolver {}
pub struct EnvVarResolver {}
pub struct CredentialResolver {
    credentials: HashMap<String, String>,
}
impl CredentialResolver {
    pub fn new(credentials: HashMap<String, String>) -> Self {
        CredentialResolver { credentials }
    }
}

pub fn load_engine_config(
    config_content: &str,
    engine_name: &str,
    overrides: &HashMap<String, Value>,
    credentials: &HashMap<String, String>,
) -> Result<EngineConfig> {
    //Converts the string into a json value to be manipulated
    let mut config: Value = serde_json::from_str(config_content)?;

    debug!("Loading config for engine: {}", engine_name);

    // Find the specific engine configuration
    let engine_config = config["engines"]
        .as_array_mut()
        .ok_or_else(|| anyhow!("No engines found in configuration"))?
        .iter_mut()
        .find(|e| e["name"].as_str() == Some(engine_name))
        .ok_or_else(|| anyhow!("Engine '{}' not found in configuration", engine_name))?;

    // Resolve variables
    apply_variable_resolver(engine_config, credentials)?;

    // Override variables
    apply_variable_overrider(engine_config, overrides)?;

    // Apply overrides to the specified engine
    /*
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
    */

    debug!("Loaded and processed config for engine: {}", engine_name);

    serde_json::from_value(engine_config.clone()).context("Could not parse engine config")
}

fn apply_variable_resolver(
    engine_config: &mut Value,
    credentials: &HashMap<String, String>,
) -> Result<()> {
    let mut processor = VariableResolverProcessor::new(credentials);
    processor.resolve(engine_config)?;
    Ok(())
}
fn apply_variable_overrider(
    engine_config: &mut Value,
    overrides: &HashMap<String, Value>,
) -> Result<()> {
    if let Some(parameters) = engine_config
        .get_mut("parameters")
        .and_then(Value::as_object_mut)
    {
        for (key, value) in overrides {
            // Split the key into parts to handle nested paths
            let mut keys = key.split('.').peekable();
            // Traverse the parameters to find the correct nested object
            let mut current = &mut *parameters; // Reborrow parameters for each iteration
            while let Some(part) = keys.next() {
                if keys.peek().is_none() {
                    current.insert(part.to_string(), value.clone());
                } else {
                    // Continue traversing or create new nested object
                    current = current
                        .entry(part)
                        .or_insert_with(|| Value::Object(serde_json::Map::new()))
                        .as_object_mut()
                        .ok_or_else(|| anyhow!("Failed to create nested object"))?;
                }
            }
        }
    }
    Ok(())
}

pub fn load_config(
    config_path: &str,
    engine_name: &str,
    overrides: &HashMap<String, String>,
) -> Result<Config> {
    //Workaround to transform a HashMap<String,String> from cli into HashMap<String,Value>
    //This is for cli/lambda compatibility
    let overrides: HashMap<String, Value> = overrides
        .clone()
        .drain()
        .map(|(k, v)| match v.parse::<bool>() {
            Ok(b) => (k, serde_json::Value::Bool(b)),
            _ => match v.parse::<f64>() {
                Ok(f) => (
                    k,
                    serde_json::Value::Number(serde_json::Number::from_f64(f).unwrap()),
                ),
                _ => (k, serde_json::Value::String(v.clone())),
            },
        })
        .collect();
    let engine_config = load_engine_config(
        &fs::read_to_string(config_path)?,
        engine_name,
        &overrides,
        &HashMap::new(),
    )?;
    Ok(Config::new(vec![engine_config]))
}

impl VariableResolver for CredentialResolver {
    fn is_resolvable(&self, key: &str) -> bool {
        key.starts_with("CREDENTIAL_")
    }
    fn resolve(&self, key: &str) -> Result<String> {
        let credential_key = &key[11..]; // Skip the "CREDENTIAL_" prefix to fetch the correct credential
        debug!("Attempting to replace: {}", key);
        debug!("Looking up credential: {}", credential_key);
        match self.credentials.get(credential_key) {
            Some(credential_value) => {
                debug!("Credential found for: {}", credential_key);
                Ok(credential_value.clone())
            }
            None => {
                debug!("Failed to find credential '{}'", credential_key);
                Err(anyhow!("Failed to find credential '{}'", credential_key))
            }
        }
    }
}
impl VariableResolver for AmberVarResolver {
    fn is_resolvable(&self, key: &str) -> bool {
        key.starts_with("AMBER_")
    }
    fn resolve(&self, key: &str) -> Result<String> {
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

impl VariableResolver for EnvVarResolver {
    fn is_resolvable(&self, key: &str) -> bool {
        key.starts_with("ENV_")
    }
    fn resolve(&self, key: &str) -> Result<String> {
        let env_key = &key[4..]; // Skip the "ENV_" prefix to fetch the correct env var
        debug!("Attempting to replace: {}", key);
        debug!("Looking up environment variable: {}", env_key);
        match env::var(env_key) {
            Ok(env_value) => {
                debug!("Environment value found for: {}", env_key);
                Ok(env_value)
            }
            Err(e) => {
                debug!("Failed to find environment variable '{}': {}", env_key, e);
                Err(anyhow!(
                    "Failed to find environment variable '{}': {}",
                    env_key,
                    e
                ))
            }
        }
    }
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

impl Default for VariableResolverProcessor {
    fn default() -> Self {
        VariableResolverProcessor {
            keys: Vec::new(),
            resolvers: vec![Arc::new(EnvVarResolver {}), Arc::new(AmberVarResolver {})],
        }
    }
}

pub struct VariableResolverProcessor {
    keys: Vec<String>,
    resolvers: Vec<Arc<dyn VariableResolver>>,
}

impl VariableResolverProcessor {
    pub fn new(credentials: &HashMap<String, String>) -> Self {
        VariableResolverProcessor {
            keys: Vec::new(),
            resolvers: vec![
                Arc::new(EnvVarResolver {}),
                Arc::new(AmberVarResolver {}),
                Arc::new(CredentialResolver::new(credentials.clone())),
            ],
        }
    }
    fn resolve(&mut self, value: &mut Value) -> Result<()> {
        match value {
            Value::String(s) => {
                for resolver in &self.resolvers {
                    if resolver.is_resolvable(s) {
                        let resolved = resolver.resolve(s)?;
                        self.set_env_var_from_amber(s, &resolved)?; //TODO: Is this necessary?
                        *s = resolved;
                        return Ok(());
                    }
                }
                Ok(())
            }
            Value::Object(map) => {
                for (_, v) in map.iter_mut() {
                    self.resolve(v)?;
                }
                Ok(())
            }
            Value::Array(arr) => {
                for item in arr.iter_mut() {
                    self.resolve(item)?;
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
}

impl Drop for VariableResolverProcessor {
    fn drop(&mut self) {
        for key in &self.keys {
            std::env::remove_var(key);
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
