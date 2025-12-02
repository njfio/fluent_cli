/// Tests for missing API key error handling
///
/// This test suite validates that all engines produce clear, user-friendly error messages
/// when API keys are missing from the configuration.

use fluent_core::config::{ConnectionConfig, EngineConfig};
use fluent_engines::*;
use std::collections::HashMap;
use std::pin::Pin;

/// Helper function to create a basic engine config without API keys
fn create_config_without_api_key(engine_type: &str) -> EngineConfig {
    EngineConfig {
        name: "test".to_string(),
        engine: engine_type.to_string(),
        connection: ConnectionConfig {
            protocol: "https".to_string(),
            hostname: "api.example.com".to_string(),
            port: 443,
            request_path: "/v1/chat/completions".to_string(),
        },
        parameters: HashMap::new(), // Empty parameters - no API key
        session_id: None,
        neo4j: None,
        spinner: None,
    }
}

#[tokio::test]
async fn test_openai_missing_api_key() {
    let config = create_config_without_api_key("openai");
    let result = openai::OpenAIEngine::new(config).await;

    assert!(result.is_err(), "OpenAI engine should fail without API key");

    let err_msg = match result {
        Err(e) => e.to_string(),
        Ok(_) => panic!("Expected error but got success"),
    };
    assert!(
        err_msg.to_lowercase().contains("openai"),
        "Error message should mention OpenAI: {}",
        err_msg
    );
    assert!(
        err_msg.to_lowercase().contains("api key") || err_msg.to_lowercase().contains("api_key"),
        "Error message should mention 'API key': {}",
        err_msg
    );
    assert!(
        err_msg.contains("OPENAI_API_KEY") || err_msg.to_lowercase().contains("environment variable"),
        "Error message should mention environment variable or OPENAI_API_KEY: {}",
        err_msg
    );
}

#[tokio::test]
async fn test_anthropic_missing_api_key() {
    let mut config = create_config_without_api_key("anthropic");
    // Anthropic requires a modelName parameter
    config.parameters.insert("modelName".to_string(), serde_json::json!("claude-sonnet-4-20250514"));

    // Anthropic doesn't fail on initialization, so create engine first
    let engine = anthropic::AnthropicEngine::new(config).await;

    if let Ok(engine) = engine {
        use fluent_core::traits::Engine;
        use fluent_core::types::Request;

        let request = Request {
            flowname: "test".to_string(),
            payload: "test".to_string(),
        };

        let future = engine.execute(&request);
        let result = Pin::from(future).await;
        assert!(result.is_err(), "Request should fail without API key");

        let err_msg = match result {
            Err(e) => e.to_string(),
            Ok(_) => panic!("Expected error but got success"),
        };
        assert!(
            err_msg.to_lowercase().contains("anthropic"),
            "Error message should mention Anthropic: {}",
            err_msg
        );
        assert!(
            err_msg.to_lowercase().contains("api key") || err_msg.to_lowercase().contains("api_key"),
            "Error message should mention 'API key': {}",
            err_msg
        );
    }
}

#[tokio::test]
async fn test_google_gemini_missing_api_key() {
    let config = create_config_without_api_key("google_gemini");

    // Google Gemini doesn't fail on initialization, but on first request
    // So we test the error message by trying to send a request
    let engine = google_gemini::GoogleGeminiEngine::new(config).await;

    // The engine creation might succeed but requests will fail
    if let Ok(engine) = engine {
        use fluent_core::traits::Engine;
        use fluent_core::types::Request;

        let request = Request {
            flowname: "test".to_string(),
            payload: "test".to_string(),
        };

        let future = engine.execute(&request);
        let result = Pin::from(future).await;
        assert!(result.is_err(), "Request should fail without API key");

        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.to_lowercase().contains("gemini") || err_msg.to_lowercase().contains("google"),
            "Error message should mention Google/Gemini: {}",
            err_msg
        );
    } else {
        // If it fails on creation, that's also valid
        let err_msg = match engine {
            Err(e) => e.to_string(),
            Ok(_) => panic!("Expected error but got success"),
        };
        assert!(
            err_msg.to_lowercase().contains("api key") || err_msg.to_lowercase().contains("api_key"),
            "Error message should mention 'API key': {}",
            err_msg
        );
    }
}

#[tokio::test]
async fn test_cohere_missing_api_key() {
    let config = create_config_without_api_key("cohere");

    // Cohere doesn't fail on initialization, so create engine first
    let engine = cohere::CohereEngine::new(config).await;

    if let Ok(engine) = engine {
        use fluent_core::traits::Engine;
        use fluent_core::types::Request;

        let request = Request {
            flowname: "test".to_string(),
            payload: "test".to_string(),
        };

        let future = engine.execute(&request);
        let result = Pin::from(future).await;
        assert!(result.is_err(), "Request should fail without API key");

        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.to_lowercase().contains("cohere"),
            "Error message should mention Cohere: {}",
            err_msg
        );
        assert!(
            err_msg.to_lowercase().contains("api key") || err_msg.to_lowercase().contains("api_key"),
            "Error message should mention 'API key': {}",
            err_msg
        );
    }
}

#[tokio::test]
async fn test_mistral_missing_api_key() {
    let config = create_config_without_api_key("mistral");

    // Mistral doesn't fail on initialization
    let engine = mistral::MistralEngine::new(config).await;

    if let Ok(engine) = engine {
        use fluent_core::traits::Engine;
        use fluent_core::types::Request;

        let request = Request {
            flowname: "test".to_string(),
            payload: "test".to_string(),
        };

        let future = engine.execute(&request);
        let result = Pin::from(future).await;
        assert!(result.is_err(), "Request should fail without API key");

        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.to_lowercase().contains("mistral"),
            "Error message should mention Mistral: {}",
            err_msg
        );
        assert!(
            err_msg.to_lowercase().contains("api key") || err_msg.to_lowercase().contains("api_key"),
            "Error message should mention 'API key': {}",
            err_msg
        );
    }
}

#[tokio::test]
async fn test_perplexity_missing_api_key() {
    let config = create_config_without_api_key("perplexity");

    // Perplexity doesn't fail on initialization
    let engine = perplexity::PerplexityEngine::new(config).await;

    if let Ok(engine) = engine {
        use fluent_core::traits::Engine;
        use fluent_core::types::Request;

        let request = Request {
            flowname: "test".to_string(),
            payload: "test".to_string(),
        };

        let future = engine.execute(&request);
        let result = Pin::from(future).await;
        assert!(result.is_err(), "Request should fail without API key");

        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.to_lowercase().contains("perplexity"),
            "Error message should mention Perplexity: {}",
            err_msg
        );
        assert!(
            err_msg.to_lowercase().contains("api key") || err_msg.to_lowercase().contains("api_key"),
            "Error message should mention 'API key': {}",
            err_msg
        );
    }
}

#[tokio::test]
async fn test_groq_missing_api_key() {
    let config = create_config_without_api_key("groq_lpu");

    // Groq doesn't fail on initialization
    let engine = groqlpu::GroqLPUEngine::new(config).await;

    if let Ok(engine) = engine {
        use fluent_core::traits::Engine;
        use fluent_core::types::Request;

        let request = Request {
            flowname: "test".to_string(),
            payload: "test".to_string(),
        };

        let future = engine.execute(&request);
        let result = Pin::from(future).await;
        assert!(result.is_err(), "Request should fail without API key");

        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.to_lowercase().contains("groq"),
            "Error message should mention Groq: {}",
            err_msg
        );
        assert!(
            err_msg.to_lowercase().contains("api key") || err_msg.to_lowercase().contains("api_key"),
            "Error message should mention 'API key': {}",
            err_msg
        );
    }
}

/// Test that error messages contain helpful information about how to fix the issue
#[tokio::test]
async fn test_error_messages_contain_helpful_guidance() {
    let config = create_config_without_api_key("openai");
    let result = openai::OpenAIEngine::new(config).await;

    assert!(result.is_err());
    let err_msg = match result {
        Err(e) => e.to_string(),
        Ok(_) => panic!("Expected error but got success"),
    };

    // Error should mention at least one of these helpful terms
    let has_helpful_info =
        err_msg.to_lowercase().contains("environment variable") ||
        err_msg.to_lowercase().contains("config") ||
        err_msg.contains("bearer_token") ||
        err_msg.contains("api_key") ||
        err_msg.contains("OPENAI_API_KEY");

    assert!(
        has_helpful_info,
        "Error message should provide helpful guidance on how to fix the issue: {}",
        err_msg
    );
}
