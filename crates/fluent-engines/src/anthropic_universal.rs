use crate::universal_base_engine::UniversalEngine;
use anyhow::Result;
use async_trait::async_trait;
use fluent_core::config::EngineConfig;
use fluent_core::neo4j_client::Neo4jClient;
use fluent_core::traits::Engine;
use fluent_core::types::{ExtractedContent, Request, Response, UpsertRequest, UpsertResponse};
use serde_json::Value;
use std::future::Future;
use std::path::Path;
use std::sync::Arc;

/// Anthropic engine implementation using the universal base engine
/// This demonstrates how to migrate existing engines to use the universal base
pub struct AnthropicUniversalEngine {
    universal: UniversalEngine,
}

impl AnthropicUniversalEngine {
    /// Create a new Anthropic engine using the universal base
    pub async fn new(config: EngineConfig) -> Result<Self> {
        let universal = UniversalEngine::anthropic(config).await?;
        Ok(Self { universal })
    }
}

#[async_trait]
impl Engine for AnthropicUniversalEngine {
    fn execute<'a>(
        &'a self,
        request: &'a Request,
    ) -> Box<dyn Future<Output = Result<Response>> + Send + 'a> {
        // Delegate to the universal engine - all the common functionality is handled automatically
        self.universal.execute(request)
    }

    fn upsert<'a>(
        &'a self,
        request: &'a UpsertRequest,
    ) -> Box<dyn Future<Output = Result<UpsertResponse>> + Send + 'a> {
        self.universal.upsert(request)
    }

    fn get_neo4j_client(&self) -> Option<&Arc<Neo4jClient>> {
        self.universal.get_neo4j_client()
    }

    fn get_session_id(&self) -> Option<String> {
        self.universal.get_session_id()
    }

    fn extract_content(&self, value: &Value) -> Option<ExtractedContent> {
        self.universal.extract_content(value)
    }

    fn upload_file<'a>(
        &'a self,
        file_path: &'a Path,
    ) -> Box<dyn Future<Output = Result<String>> + Send + 'a> {
        self.universal.upload_file(file_path)
    }

    fn process_request_with_file<'a>(
        &'a self,
        request: &'a Request,
        file_path: &'a Path,
    ) -> Box<dyn Future<Output = Result<Response>> + Send + 'a> {
        self.universal.process_request_with_file(request, file_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fluent_core::config::{ConnectionConfig, EngineConfig};
    use serde_json::json;

    fn create_anthropic_config() -> EngineConfig {
        EngineConfig {
            name: "anthropic-test".to_string(),
            engine: "anthropic".to_string(),
            connection: ConnectionConfig {
                protocol: "https".to_string(),
                hostname: "api.anthropic.com".to_string(),
                port: 443,
                request_path: "/v1/messages".to_string(),
            },
            parameters: {
                let mut params = std::collections::HashMap::new();
                params.insert("bearer_token".to_string(), json!("test-token"));
                params.insert("model".to_string(), json!("claude-sonnet-4-20250514"));
                params.insert("max_tokens".to_string(), json!(1000));
                params
            },
            session_id: None,
            neo4j: None,
            spinner: None,
        }
    }

    #[tokio::test]
    async fn test_anthropic_universal_engine_creation() {
        let config = create_anthropic_config();
        let engine = AnthropicUniversalEngine::new(config).await.unwrap();

        // Test that the engine was created successfully
        assert!(engine.get_session_id().is_none());
        assert!(engine.get_neo4j_client().is_none());
    }

    #[tokio::test]
    async fn test_anthropic_universal_engine_methods() {
        let config = create_anthropic_config();
        let engine = AnthropicUniversalEngine::new(config).await.unwrap();

        // Test that all required methods are available
        assert!(engine.get_session_id().is_none());
        assert!(engine.get_neo4j_client().is_none());

        // Test extract_content with empty value
        let empty_value = json!({});
        assert!(engine.extract_content(&empty_value).is_none());
    }
}

/// Migration guide for existing engines:
///
/// 1. Replace the engine struct with a wrapper around UniversalEngine:
///    ```rust
///    pub struct MyEngine {
///        universal: UniversalEngine,
///    }
///    ```
///
/// 2. Update the constructor to use the appropriate universal engine factory:
///    ```rust
///    pub async fn new(config: EngineConfig) -> Result<Self> {
///        let universal = UniversalEngine::openai(config).await?; // or anthropic, google_gemini
///        Ok(Self { universal })
///    }
///    ```
///
/// 3. Implement the Engine trait by delegating to the universal engine:
///    ```rust
///    #[async_trait]
///    impl Engine for MyEngine {
///        fn execute<'a>(&'a self, request: &'a Request) -> Box<dyn Future<Output = Result<Response>> + Send + 'a> {
///            self.universal.execute(request)
///        }
///        // ... delegate other methods similarly
///    }
///    ```
///
/// Benefits of migration:
/// - Eliminates code duplication (HTTP client management, caching, error handling)
/// - Consistent behavior across all engines
/// - Automatic caching integration
/// - Standardized authentication handling
/// - Reduced maintenance burden
/// - Better performance through optimized HTTP client reuse
///
/// The universal base engine handles:
/// - HTTP client creation and optimization
/// - URL construction
/// - Authentication headers
/// - Request/response caching
/// - Error handling and retries
/// - Cost calculation
/// - Neo4j integration
/// - Response parsing for common formats
///
/// Engines only need to focus on their unique business logic and API-specific requirements.
pub struct _MigrationGuide;
