use fluent_core::config::{apply_overrides, load_engine_config, parse_key_value_pair};
use serde_json::json;
use std::collections::HashMap;

#[test]
fn test_parse_key_value_pair() {
    assert_eq!(parse_key_value_pair("k=v"), Some(("k".into(), "v".into())));
    assert_eq!(parse_key_value_pair("k="), Some(("k".into(), "".into())));
    assert_eq!(parse_key_value_pair("invalid"), None);
}

#[test]
fn test_apply_overrides_inserts_values() {
    let mut cfg = fluent_core::config::EngineConfig {
        name: "n".into(),
        engine: "openai".into(),
        connection: fluent_core::config::ConnectionConfig {
            protocol: "https".into(),
            hostname: "h".into(),
            port: 443,
            request_path: "/".into(),
        },
        parameters: HashMap::new(),
        session_id: None,
        neo4j: None,
        spinner: None,
    };
    apply_overrides(&mut cfg, &[("max_tokens".into(), "123".into())]).unwrap();
    assert_eq!(cfg.parameters.get("max_tokens"), Some(&json!(123)));
}

#[test]
fn test_load_engine_config_with_credentials_and_env() {
    let yaml = r#"
engines:
  - name: test
    engine: openai
    connection:
      protocol: https
      hostname: api.example.com
      port: 443
      request_path: /v1
    parameters:
      api_key: CREDENTIAL_API_KEY
      region: ENV_REGION
"#;
    let mut overrides = HashMap::new();
    overrides.insert("temperature".to_string(), json!(0.2));

    let mut creds = HashMap::new();
    creds.insert("API_KEY".to_string(), "SECRET".to_string());
    std::env::set_var("REGION", "us-east1");

    let cfg = load_engine_config(yaml, "test", &overrides, &creds).unwrap();
    assert_eq!(cfg.parameters.get("api_key"), Some(&json!("SECRET")));
    assert_eq!(cfg.parameters.get("region"), Some(&json!("us-east1")));
    assert_eq!(cfg.parameters.get("temperature"), Some(&json!(0.2)));
}
