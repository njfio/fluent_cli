use anyhow::Result;
use fluent_core::config::{ConnectionConfig, EngineConfig};
use fluent_core::types::Request;
use fluent_engines::create_engine;

fn network_tests_enabled() -> bool {
    std::env::var("FLUENT_NETWORK_TESTS").ok().as_deref() == Some("1")
}

fn has_env(name: &str) -> bool {
    std::env::var(name).ok().filter(|v| !v.is_empty()).is_some()
}

#[tokio::test]
#[ignore]
async fn openai_basic_integration() -> Result<()> {
    if !network_tests_enabled() {
        eprintln!("Skipping: FLUENT_NETWORK_TESTS != 1");
        return Ok(());
    }
    if !has_env("OPENAI_API_KEY") {
        eprintln!("Skipping: OPENAI_API_KEY not set");
        return Ok(());
    }

    let mut params = std::collections::HashMap::new();
    params.insert("api_key".to_string(), serde_json::json!(std::env::var("OPENAI_API_KEY").unwrap()));
    params.insert(
        "model".to_string(),
        serde_json::json!(std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string())),
    );
    params.insert("temperature".to_string(), serde_json::json!(0.0));

    let cfg = EngineConfig {
        name: "openai-test".to_string(),
        engine: "openai".to_string(),
        connection: ConnectionConfig {
            protocol: std::env::var("OPENAI_PROTOCOL").unwrap_or_else(|_| "https".to_string()),
            hostname: std::env::var("OPENAI_HOSTNAME").unwrap_or_else(|_| "api.openai.com".to_string()),
            port: std::env::var("OPENAI_PORT").ok().and_then(|s| s.parse().ok()).unwrap_or(443),
            request_path: std::env::var("OPENAI_PATH").unwrap_or_else(|_| "/v1/chat/completions".to_string()),
        },
        parameters: params,
        session_id: None,
        neo4j: None,
        spinner: None,
    };

    let engine = create_engine(&cfg).await?;
    let req = Request { flowname: "integration".to_string(), payload: "Reply with the word PING".to_string() };
    let resp = std::pin::Pin::from(engine.execute(&req)).await?;
    assert!(!resp.content.trim().is_empty());
    Ok(())
}
