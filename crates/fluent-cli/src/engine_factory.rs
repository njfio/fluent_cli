//! Engine creation and management
//!
//! This module handles the creation and configuration of LLM engines,
//! including engine selection, configuration loading, and initialization.

use anyhow::{anyhow, Error, Result};
use fluent_core::config::{Config, EngineConfig};
use fluent_core::traits::Engine;
use fluent_engines::anthropic::AnthropicEngine;
use fluent_engines::openai::OpenAIEngine;

use std::pin::Pin;

/// Create an LLM engine based on the provided configuration
pub async fn create_llm_engine(engine_config: &EngineConfig) -> Result<Box<dyn Engine>, Error> {
    match engine_config.engine.as_str() {
        "openai" => {
            let engine = OpenAIEngine::new(engine_config.clone()).await?;
            Ok(Box::new(engine))
        }
        "anthropic" => {
            let engine = AnthropicEngine::new(engine_config.clone()).await?;
            Ok(Box::new(engine))
        }
        _ => Err(anyhow!(
            "Unsupported engine type: {}",
            engine_config.engine
        )),
    }
}

/// Get the Neo4j query LLM engine from configuration
pub async fn get_neo4j_query_llm(config: &Config) -> Option<(Box<dyn Engine>, &EngineConfig)> {
    // For now, just use the first available engine
    // In the future, this could be configurable
    if let Some(engine_config) = config.engines.first() {
        if let Ok(engine) = create_llm_engine(engine_config).await {
            return Some((engine, engine_config));
        }
    }
    None
}

/// Generate a Cypher query using the configured LLM
pub async fn generate_cypher_query(query: &str, config: &EngineConfig) -> Result<String> {
    let engine = create_llm_engine(config).await?;
    
    let cypher_prompt = format!(
        "Convert this natural language query to Cypher for Neo4j: {}
        
        Rules:
        1. Return only the Cypher query, no explanations
        2. Use proper Cypher syntax
        3. Be specific and efficient
        4. Handle edge cases appropriately
        
        Cypher query:",
        query
    );

    let request = fluent_core::types::Request {
        flowname: "cypher_generation".to_string(),
        payload: cypher_prompt,
    };

    let response = Pin::from(engine.execute(&request)).await?;
    
    // Extract just the Cypher query from the response
    let cypher = response.content.trim();
    
    // Basic validation - ensure it looks like a Cypher query
    if cypher.to_uppercase().contains("MATCH") 
        || cypher.to_uppercase().contains("CREATE") 
        || cypher.to_uppercase().contains("MERGE") {
        Ok(cypher.to_string())
    } else {
        Err(anyhow!("Generated response doesn't appear to be a valid Cypher query: {}", cypher))
    }
}

/// Validate engine configuration
pub fn validate_engine_config(config: &EngineConfig) -> Result<()> {
    if config.engine.is_empty() {
        return Err(anyhow!("Engine type cannot be empty"));
    }

    // Check if API key is available in parameters
    if config.parameters.get("api_key").is_none() && config.engine != "local" {
        return Err(anyhow!("API key is required for engine type: {}", config.engine));
    }

    // Validate parameters if they exist
    if let Some(max_tokens) = config.parameters.get("max_tokens") {
        if let Some(max_tokens_num) = max_tokens.as_u64() {
            if max_tokens_num == 0 {
                return Err(anyhow!("Max tokens must be greater than 0"));
            }
        }
    }

    if let Some(temperature) = config.parameters.get("temperature") {
        if let Some(temp_num) = temperature.as_f64() {
            if !(0.0..=2.0).contains(&temp_num) {
                return Err(anyhow!("Temperature must be between 0.0 and 2.0"));
            }
        }
    }

    Ok(())
}

/// Get available engine types
pub fn get_available_engines() -> Vec<&'static str> {
    vec![
        "openai",
        "anthropic",
        "gemini",
        "mistral",
        "cohere",
        "local",
    ]
}

/// Check if an engine type is supported
pub fn is_engine_supported(engine_type: &str) -> bool {
    get_available_engines().contains(&engine_type)
}

/// Create a default engine configuration for testing
#[cfg(test)]
pub fn create_test_engine_config(engine_type: &str) -> EngineConfig {
    use fluent_core::config::ConnectionConfig;
    use std::collections::HashMap;

    let mut parameters = HashMap::new();
    parameters.insert("api_key".to_string(), serde_json::Value::String("test-key".to_string()));
    parameters.insert("model".to_string(), serde_json::Value::String("test-model".to_string()));
    parameters.insert("max_tokens".to_string(), serde_json::Value::Number(serde_json::Number::from(1000)));
    parameters.insert("temperature".to_string(), serde_json::Value::Number(
        serde_json::Number::from_f64(0.7)
            .ok_or_else(|| anyhow!("Failed to create temperature number from f64"))?
    ));

    EngineConfig {
        name: format!("test-{}", engine_type),
        engine: engine_type.to_string(),
        connection: ConnectionConfig {
            protocol: "https".to_string(),
            hostname: "api.test.com".to_string(),
            port: 443,
            request_path: "/v1/chat/completions".to_string(),
        },
        parameters,
        session_id: None,
        neo4j: None,
        spinner: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_engine_config() {
        let valid_config = create_test_engine_config("openai");
        assert!(validate_engine_config(&valid_config).is_ok());

        let mut invalid_config = valid_config.clone();
        invalid_config.engine = "".to_string();
        assert!(validate_engine_config(&invalid_config).is_err());

        let mut no_api_key_config = valid_config.clone();
        no_api_key_config.connection.api_key = None;
        assert!(validate_engine_config(&no_api_key_config).is_err());
    }

    #[test]
    fn test_is_engine_supported() {
        assert!(is_engine_supported("openai"));
        assert!(is_engine_supported("anthropic"));
        assert!(!is_engine_supported("unsupported"));
    }

    #[test]
    fn test_get_available_engines() {
        let engines = get_available_engines();
        assert!(engines.contains(&"openai"));
        assert!(engines.contains(&"anthropic"));
        assert!(engines.len() >= 2);
    }
}
