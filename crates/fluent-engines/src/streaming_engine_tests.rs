#[cfg(test)]
mod comprehensive_streaming_tests {
    use crate::streaming_engine::*;
    use fluent_core::config::{ConnectionConfig, EngineConfig};
    use fluent_core::types::Request;
    use futures::StreamExt;
    use serde_json::json;
    use std::collections::HashMap;

    fn create_test_config() -> EngineConfig {
        EngineConfig {
            name: "test-streaming".to_string(),
            engine: "openai".to_string(),
            connection: ConnectionConfig {
                protocol: "https".to_string(),
                hostname: "api.openai.com".to_string(),
                port: 443,
                request_path: "/v1/chat/completions".to_string(),
            },
            parameters: {
                let mut params = HashMap::new();
                params.insert("bearer_token".to_string(), json!("test-token"));
                params.insert("model".to_string(), json!("gpt-4"));
                params.insert("temperature".to_string(), json!(0.7));
                params.insert("max_tokens".to_string(), json!(1000));
                params
            },
            session_id: None,
            neo4j: None,
            spinner: None,
        }
    }

    fn create_test_request() -> Request {
        Request {
            flowname: "test".to_string(),
            payload: "Hello, how are you?".to_string(),
        }
    }

    #[test]
    fn test_stream_chunk_creation() {
        let chunk = StreamChunk {
            id: "test-123".to_string(),
            content: "Hello".to_string(),
            is_final: false,
            token_usage: Some(ChunkTokenUsage {
                prompt_tokens: Some(10),
                completion_tokens: Some(5),
                total_tokens: Some(15),
            }),
            model: Some("gpt-4".to_string()),
            finish_reason: None,
        };

        assert_eq!(chunk.id, "test-123");
        assert_eq!(chunk.content, "Hello");
        assert!(!chunk.is_final);
        assert!(chunk.token_usage.is_some());
        assert_eq!(chunk.model, Some("gpt-4".to_string()));
        assert!(chunk.finish_reason.is_none());
    }

    #[test]
    fn test_streaming_config_default() {
        let config = StreamingConfig::default();

        assert!(config.enabled);
        assert_eq!(config.buffer_size, 8192);
        assert_eq!(config.chunk_timeout_ms, 5000);
        assert_eq!(config.max_buffered_chunks, 100);
        assert!(config.include_token_usage);
    }

    #[test]
    fn test_streaming_config_custom() {
        let config = StreamingConfig {
            enabled: false,
            buffer_size: 4096,
            chunk_timeout_ms: 3000,
            max_buffered_chunks: 50,
            include_token_usage: false,
        };

        assert!(!config.enabled);
        assert_eq!(config.buffer_size, 4096);
        assert_eq!(config.chunk_timeout_ms, 3000);
        assert_eq!(config.max_buffered_chunks, 50);
        assert!(!config.include_token_usage);
    }

    #[test]
    fn test_openai_chunk_parsing_valid() {
        let chunk_line = r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1677652288,"model":"gpt-4","choices":[{"delta":{"content":"Hello"},"index":0,"finish_reason":null}]}"#;

        let result = OpenAIStreaming::parse_openai_chunk(chunk_line).unwrap();
        assert!(result.is_some());

        let chunk = result.unwrap();
        assert_eq!(chunk.id, "chatcmpl-123");
        assert_eq!(chunk.content, "Hello");
        assert!(!chunk.is_final);
        assert_eq!(chunk.model, Some("gpt-4".to_string()));
        assert!(chunk.finish_reason.is_none());
    }

    #[test]
    fn test_openai_chunk_parsing_done() {
        let done_line = "data: [DONE]";

        let result = OpenAIStreaming::parse_openai_chunk(done_line).unwrap();
        assert!(result.is_some());

        let chunk = result.unwrap();
        assert_eq!(chunk.id, "final");
        assert_eq!(chunk.content, "");
        assert!(chunk.is_final);
        assert_eq!(chunk.finish_reason, Some("stop".to_string()));
    }

    #[test]
    fn test_openai_chunk_parsing_empty_choices() {
        let chunk_line = r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1677652288,"model":"gpt-4","choices":[]}"#;

        let result = OpenAIStreaming::parse_openai_chunk(chunk_line).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_openai_chunk_parsing_invalid_json() {
        let invalid_line = "data: {invalid json}";

        let result = OpenAIStreaming::parse_openai_chunk(invalid_line);
        assert!(result.is_err());
    }

    #[test]
    fn test_openai_chunk_parsing_no_data_prefix() {
        let no_prefix_line = r#"{"id":"chatcmpl-123","choices":[{"delta":{"content":"Hello"}}]}"#;

        let result = OpenAIStreaming::parse_openai_chunk(no_prefix_line).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_openai_chunk_parsing_with_finish_reason() {
        let chunk_line = r#"data: {"id":"chatcmpl-123","model":"gpt-4","choices":[{"delta":{"content":""},"index":0,"finish_reason":"stop"}]}"#;

        let result = OpenAIStreaming::parse_openai_chunk(chunk_line).unwrap();
        assert!(result.is_some());

        let chunk = result.unwrap();
        assert_eq!(chunk.content, "");
        assert!(chunk.is_final);
        assert_eq!(chunk.finish_reason, Some("stop".to_string()));
    }

    #[test]
    fn test_openai_chunk_parsing_with_usage() {
        let chunk_line = r#"data: {"id":"chatcmpl-123","model":"gpt-4","choices":[{"delta":{"content":"Hello"}}],"usage":{"prompt_tokens":10,"completion_tokens":5,"total_tokens":15}}"#;

        let result = OpenAIStreaming::parse_openai_chunk(chunk_line).unwrap();
        assert!(result.is_some());

        let chunk = result.unwrap();
        assert!(chunk.token_usage.is_some());
        let usage = chunk.token_usage.unwrap();
        assert_eq!(usage.prompt_tokens, Some(10));
        assert_eq!(usage.completion_tokens, Some(5));
        assert_eq!(usage.total_tokens, Some(15));
    }

    #[test]
    fn test_anthropic_chunk_parsing_content_delta() {
        let chunk_line =
            r#"data: {"type":"content_block_delta","index":0,"delta":{"text":"Hello"}}"#;

        let result = AnthropicStreaming::parse_anthropic_chunk(chunk_line).unwrap();
        assert!(result.is_some());

        let chunk = result.unwrap();
        assert_eq!(chunk.id, "0");
        assert_eq!(chunk.content, "Hello");
        assert!(!chunk.is_final);
    }

    #[test]
    fn test_anthropic_chunk_parsing_message_stop() {
        let chunk_line = r#"data: {"type":"message_stop"}"#;

        let result = AnthropicStreaming::parse_anthropic_chunk(chunk_line).unwrap();
        assert!(result.is_some());

        let chunk = result.unwrap();
        assert_eq!(chunk.id, "final");
        assert_eq!(chunk.content, "");
        assert!(chunk.is_final);
        assert_eq!(chunk.finish_reason, Some("end_turn".to_string()));
    }

    #[test]
    fn test_anthropic_chunk_parsing_unknown_type() {
        let chunk_line = r#"data: {"type":"unknown_type","data":"test"}"#;

        let result = AnthropicStreaming::parse_anthropic_chunk(chunk_line).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_anthropic_chunk_parsing_done() {
        let done_line = "data: [DONE]";

        let result = AnthropicStreaming::parse_anthropic_chunk(done_line).unwrap();
        assert!(result.is_some());

        let chunk = result.unwrap();
        assert!(chunk.is_final);
        assert_eq!(chunk.finish_reason, Some("end_turn".to_string()));
    }

    #[tokio::test]
    async fn test_openai_streaming_creation() {
        let config = create_test_config();
        let client = reqwest::Client::new();
        let streaming = OpenAIStreaming::new(client, config);

        assert!(streaming.supports_streaming());

        let streaming_config = streaming.get_streaming_config();
        assert!(streaming_config.enabled);
    }

    #[tokio::test]
    async fn test_anthropic_streaming_creation() {
        let config = create_test_config();
        let client = reqwest::Client::new();
        let streaming = AnthropicStreaming::new(client, config);

        assert!(streaming.supports_streaming());

        let streaming_config = streaming.get_streaming_config();
        assert!(streaming_config.enabled);
    }

    #[tokio::test]
    async fn test_streaming_utils_collect_empty_stream() {
        use futures::stream;

        let empty_stream: ResponseStream = Box::pin(stream::empty());
        let result = StreamingUtils::collect_stream(empty_stream).await.unwrap();

        assert_eq!(result.content, "");
        assert_eq!(result.usage.total_tokens, 0);
        assert_eq!(result.model, "");
        assert!(result.finish_reason.is_none());
    }

    #[tokio::test]
    async fn test_streaming_utils_collect_single_chunk() {
        use futures::stream;

        let chunk = StreamChunk {
            id: "test".to_string(),
            content: "Hello World".to_string(),
            is_final: true,
            token_usage: Some(ChunkTokenUsage {
                prompt_tokens: Some(5),
                completion_tokens: Some(10),
                total_tokens: Some(15),
            }),
            model: Some("gpt-4".to_string()),
            finish_reason: Some("stop".to_string()),
        };

        let stream: ResponseStream = Box::pin(stream::once(async { Ok(chunk) }));
        let result = StreamingUtils::collect_stream(stream).await.unwrap();

        assert_eq!(result.content, "Hello World");
        assert_eq!(result.usage.prompt_tokens, 5);
        assert_eq!(result.usage.completion_tokens, 10);
        assert_eq!(result.usage.total_tokens, 15);
        assert_eq!(result.model, "gpt-4");
        assert_eq!(result.finish_reason, Some("stop".to_string()));
    }

    #[tokio::test]
    async fn test_streaming_utils_collect_multiple_chunks() {
        use futures::stream;

        let chunks = vec![
            Ok(StreamChunk {
                id: "1".to_string(),
                content: "Hello ".to_string(),
                is_final: false,
                token_usage: Some(ChunkTokenUsage {
                    prompt_tokens: Some(5),
                    completion_tokens: Some(3),
                    total_tokens: Some(8),
                }),
                model: Some("gpt-4".to_string()),
                finish_reason: None,
            }),
            Ok(StreamChunk {
                id: "2".to_string(),
                content: "World".to_string(),
                is_final: true,
                token_usage: Some(ChunkTokenUsage {
                    prompt_tokens: Some(5),
                    completion_tokens: Some(7),
                    total_tokens: Some(12),
                }),
                model: Some("gpt-4".to_string()),
                finish_reason: Some("stop".to_string()),
            }),
        ];

        let stream: ResponseStream = Box::pin(stream::iter(chunks));
        let result = StreamingUtils::collect_stream(stream).await.unwrap();

        assert_eq!(result.content, "Hello World");
        assert_eq!(result.usage.prompt_tokens, 5); // Last prompt tokens
        assert_eq!(result.usage.completion_tokens, 10); // Sum of completion tokens
        assert_eq!(result.usage.total_tokens, 15); // prompt + completion
        assert_eq!(result.model, "gpt-4");
        assert_eq!(result.finish_reason, Some("stop".to_string()));
    }

    #[test]
    fn test_streaming_utils_progress_callback() {
        let mut collected_chunks = Vec::new();

        let mut callback = StreamingUtils::create_progress_callback(|chunk: &str| {
            collected_chunks.push(chunk.to_string());
        });

        let chunk1 = StreamChunk {
            id: "1".to_string(),
            content: "Hello ".to_string(),
            is_final: false,
            token_usage: None,
            model: None,
            finish_reason: None,
        };

        let chunk2 = StreamChunk {
            id: "2".to_string(),
            content: "World".to_string(),
            is_final: true,
            token_usage: None,
            model: None,
            finish_reason: Some("stop".to_string()),
        };

        callback(chunk1).unwrap();
        callback(chunk2).unwrap();

        assert_eq!(collected_chunks, vec!["Hello ", "World"]);
    }

    #[test]
    fn test_chunk_token_usage_creation() {
        let usage = ChunkTokenUsage {
            prompt_tokens: Some(100),
            completion_tokens: Some(50),
            total_tokens: Some(150),
        };

        assert_eq!(usage.prompt_tokens, Some(100));
        assert_eq!(usage.completion_tokens, Some(50));
        assert_eq!(usage.total_tokens, Some(150));
    }

    #[test]
    fn test_chunk_token_usage_partial() {
        let usage = ChunkTokenUsage {
            prompt_tokens: None,
            completion_tokens: Some(25),
            total_tokens: None,
        };

        assert!(usage.prompt_tokens.is_none());
        assert_eq!(usage.completion_tokens, Some(25));
        assert!(usage.total_tokens.is_none());
    }

    // Error handling tests
    #[tokio::test]
    async fn test_streaming_utils_collect_error_stream() {
        use anyhow::anyhow;
        use futures::stream;

        let error_stream: ResponseStream =
            Box::pin(stream::once(async { Err(anyhow!("Test error")) }));

        let result = StreamingUtils::collect_stream(error_stream).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Test error"));
    }

    // Integration tests would go here but require actual HTTP servers
    // These would test the full streaming pipeline with mock servers
}
