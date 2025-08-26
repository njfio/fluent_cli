use fluent_engines::create_engine;
use fluent_core::config::{EngineConfig, ConnectionConfig};

#[tokio::test]
async fn unknown_engine_type_returns_uniform_error() {
    let cfg = EngineConfig {
        name: "test".to_string(),
        engine: "totally_unknown".to_string(),
        connection: ConnectionConfig {
            protocol: "https".to_string(),
            hostname: "example.com".to_string(),
            port: 443,
            request_path: "/v1".to_string(),
        },
        parameters: Default::default(),
        session_id: None,
        neo4j: None,
        spinner: None,
    };

    let err = create_engine(&cfg).await.err().expect("should error");
    assert!(err.to_string().to_lowercase().contains("unknown engine type"));
}
