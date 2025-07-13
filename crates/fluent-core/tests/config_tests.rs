use fluent_core::config::{Config, EngineConfig, ConnectionConfig, Neo4jConfig};
use std::collections::HashMap;
use anyhow::Result;

/// Unit tests for configuration management
/// Tests configuration creation, validation, and serialization

#[test]
fn test_engine_config_creation() -> Result<()> {
    let mut parameters = HashMap::new();
    parameters.insert("bearer_token".to_string(), serde_json::json!("test-token"));
    parameters.insert("model".to_string(), serde_json::json!("gpt-3.5-turbo"));

    let config = EngineConfig {
        name: "test_engine".to_string(),
        engine: "openai".to_string(),
        connection: ConnectionConfig {
            protocol: "https".to_string(),
            hostname: "api.openai.com".to_string(),
            port: 443,
            request_path: "/v1/chat/completions".to_string(),
        },
        parameters,
        session_id: Some("test_session".to_string()),
        neo4j: None,
        spinner: None,
    };

    assert_eq!(config.name, "test_engine");
    assert_eq!(config.engine, "openai");
    assert_eq!(config.connection.hostname, "api.openai.com");
    assert_eq!(config.connection.port, 443);
    assert!(config.session_id.is_some());
    assert!(config.neo4j.is_none());

    Ok(())
}

#[test]
fn test_config_creation() -> Result<()> {
    let mut parameters = HashMap::new();
    parameters.insert("bearer_token".to_string(), serde_json::json!("test-token"));

    let engine_config = EngineConfig {
        name: "test_engine".to_string(),
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
    };

    let config = Config::new(vec![engine_config]);
    assert_eq!(config.engines.len(), 1);
    assert_eq!(config.engines[0].name, "test_engine");

    Ok(())
}

#[test]
fn test_connection_config() -> Result<()> {
    let connection = ConnectionConfig {
        protocol: "https".to_string(),
        hostname: "api.anthropic.com".to_string(),
        port: 443,
        request_path: "/v1/messages".to_string(),
    };

    assert_eq!(connection.protocol, "https");
    assert_eq!(connection.hostname, "api.anthropic.com");
    assert_eq!(connection.port, 443);
    assert_eq!(connection.request_path, "/v1/messages");

    Ok(())
}

#[test]
fn test_neo4j_config() -> Result<()> {
    let mut parameters = HashMap::new();
    parameters.insert("embedding_model".to_string(), serde_json::json!("text-embedding-ada-002"));

    let neo4j_config = Neo4jConfig {
        uri: "bolt://localhost:7687".to_string(),
        user: "neo4j".to_string(),
        password: "password".to_string(),
        database: "neo4j".to_string(),
        voyage_ai: None,
        query_llm: Some("gpt-4".to_string()),
        parameters: Some(parameters),
    };

    assert_eq!(neo4j_config.uri, "bolt://localhost:7687");
    assert_eq!(neo4j_config.user, "neo4j");
    assert_eq!(neo4j_config.database, "neo4j");
    assert!(neo4j_config.query_llm.is_some());
    assert!(neo4j_config.parameters.is_some());

    Ok(())
}

#[test]
fn test_config_serialization() -> Result<()> {
    let mut parameters = HashMap::new();
    parameters.insert("bearer_token".to_string(), serde_json::json!("test-token"));
    parameters.insert("model".to_string(), serde_json::json!("gpt-3.5-turbo"));

    let engine_config = EngineConfig {
        name: "test_engine".to_string(),
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
    };

    // Test JSON serialization
    let json_str = serde_json::to_string(&engine_config)?;
    assert!(json_str.contains("test_engine"));
    assert!(json_str.contains("openai"));
    assert!(json_str.contains("api.openai.com"));

    // Test JSON deserialization
    let deserialized: EngineConfig = serde_json::from_str(&json_str)?;
    assert_eq!(deserialized.name, "test_engine");
    assert_eq!(deserialized.engine, "openai");
    assert_eq!(deserialized.connection.hostname, "api.openai.com");

    Ok(())
}

#[test]
fn test_multiple_engines_config() -> Result<()> {
    let mut openai_params = HashMap::new();
    openai_params.insert("bearer_token".to_string(), serde_json::json!("openai-token"));

    let mut anthropic_params = HashMap::new();
    anthropic_params.insert("api_key".to_string(), serde_json::json!("anthropic-key"));

    let openai_config = EngineConfig {
        name: "openai_engine".to_string(),
        engine: "openai".to_string(),
        connection: ConnectionConfig {
            protocol: "https".to_string(),
            hostname: "api.openai.com".to_string(),
            port: 443,
            request_path: "/v1/chat/completions".to_string(),
        },
        parameters: openai_params,
        session_id: None,
        neo4j: None,
        spinner: None,
    };

    let anthropic_config = EngineConfig {
        name: "anthropic_engine".to_string(),
        engine: "anthropic".to_string(),
        connection: ConnectionConfig {
            protocol: "https".to_string(),
            hostname: "api.anthropic.com".to_string(),
            port: 443,
            request_path: "/v1/messages".to_string(),
        },
        parameters: anthropic_params,
        session_id: None,
        neo4j: None,
        spinner: None,
    };

    let config = Config::new(vec![openai_config, anthropic_config]);
    assert_eq!(config.engines.len(), 2);
    assert_eq!(config.engines[0].name, "openai_engine");
    assert_eq!(config.engines[1].name, "anthropic_engine");

    Ok(())
}

#[test]
fn test_config_edge_cases() -> Result<()> {
    // Test empty config
    let empty_config = Config::new(vec![]);
    assert_eq!(empty_config.engines.len(), 0);

    // Test config with minimal parameters
    let minimal_config = EngineConfig {
        name: "minimal".to_string(),
        engine: "test".to_string(),
        connection: ConnectionConfig {
            protocol: "http".to_string(),
            hostname: "localhost".to_string(),
            port: 8080,
            request_path: "/".to_string(),
        },
        parameters: HashMap::new(),
        session_id: None,
        neo4j: None,
        spinner: None,
    };

    assert_eq!(minimal_config.name, "minimal");
    assert!(minimal_config.parameters.is_empty());
    assert!(minimal_config.session_id.is_none());

    Ok(())
}

#[test]
fn test_parameter_types() -> Result<()> {
    let mut parameters = HashMap::new();
    parameters.insert("string_param".to_string(), serde_json::json!("test_string"));
    parameters.insert("number_param".to_string(), serde_json::json!(42));
    parameters.insert("float_param".to_string(), serde_json::json!(3.14));
    parameters.insert("bool_param".to_string(), serde_json::json!(true));
    parameters.insert("array_param".to_string(), serde_json::json!(["a", "b", "c"]));
    parameters.insert("object_param".to_string(), serde_json::json!({"key": "value"}));

    let config = EngineConfig {
        name: "test_types".to_string(),
        engine: "test".to_string(),
        connection: ConnectionConfig {
            protocol: "https".to_string(),
            hostname: "api.test.com".to_string(),
            port: 443,
            request_path: "/v1/test".to_string(),
        },
        parameters,
        session_id: None,
        neo4j: None,
        spinner: None,
    };

    assert_eq!(config.parameters.len(), 6);
    assert!(config.parameters.contains_key("string_param"));
    assert!(config.parameters.contains_key("number_param"));
    assert!(config.parameters.contains_key("float_param"));
    assert!(config.parameters.contains_key("bool_param"));
    assert!(config.parameters.contains_key("array_param"));
    assert!(config.parameters.contains_key("object_param"));

    // Test parameter value types
    assert!(config.parameters["string_param"].is_string());
    assert!(config.parameters["number_param"].is_number());
    assert!(config.parameters["bool_param"].is_boolean());
    assert!(config.parameters["array_param"].is_array());
    assert!(config.parameters["object_param"].is_object());

    Ok(())
}
