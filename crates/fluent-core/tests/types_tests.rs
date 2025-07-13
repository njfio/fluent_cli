use fluent_core::types::{Request, Response, Usage, Cost, ExtractedContent, UpsertRequest, UpsertResponse};
use anyhow::Result;

/// Unit tests for core types
/// Tests type creation, serialization, and validation

#[test]
fn test_request_creation() -> Result<()> {
    let request = Request {
        flowname: "chat".to_string(),
        payload: "Hello, how are you?".to_string(),
    };

    assert_eq!(request.flowname, "chat");
    assert_eq!(request.payload, "Hello, how are you?");

    Ok(())
}

#[test]
fn test_request_serialization() -> Result<()> {
    let request = Request {
        flowname: "completion".to_string(),
        payload: "Complete this sentence: The weather today is".to_string(),
    };

    // Test JSON serialization
    let json_str = serde_json::to_string(&request)?;
    assert!(json_str.contains("completion"));
    assert!(json_str.contains("Complete this sentence"));

    // Test JSON deserialization
    let deserialized: Request = serde_json::from_str(&json_str)?;
    assert_eq!(deserialized.flowname, "completion");
    assert_eq!(deserialized.payload, "Complete this sentence: The weather today is");

    Ok(())
}

#[test]
fn test_response_creation() -> Result<()> {
    let usage = Usage {
        prompt_tokens: 10,
        completion_tokens: 20,
        total_tokens: 30,
    };

    let cost = Cost {
        prompt_cost: 0.001,
        completion_cost: 0.002,
        total_cost: 0.003,
    };

    let response = Response {
        content: "Hello! I'm doing well, thank you for asking.".to_string(),
        usage,
        cost,
        model: "gpt-3.5-turbo".to_string(),
        finish_reason: Some("stop".to_string()),
    };

    assert_eq!(response.content, "Hello! I'm doing well, thank you for asking.");
    assert_eq!(response.usage.prompt_tokens, 10);
    assert_eq!(response.usage.completion_tokens, 20);
    assert_eq!(response.usage.total_tokens, 30);
    assert_eq!(response.cost.prompt_cost, 0.001);
    assert_eq!(response.cost.completion_cost, 0.002);
    assert_eq!(response.cost.total_cost, 0.003);
    assert_eq!(response.model, "gpt-3.5-turbo");
    assert_eq!(response.finish_reason, Some("stop".to_string()));

    Ok(())
}

#[test]
fn test_response_serialization() -> Result<()> {
    let usage = Usage {
        prompt_tokens: 15,
        completion_tokens: 25,
        total_tokens: 40,
    };

    let cost = Cost {
        prompt_cost: 0.0015,
        completion_cost: 0.0025,
        total_cost: 0.004,
    };

    let response = Response {
        content: "This is a test response.".to_string(),
        usage,
        cost,
        model: "gpt-4".to_string(),
        finish_reason: Some("stop".to_string()),
    };

    // Test JSON serialization
    let json_str = serde_json::to_string(&response)?;
    assert!(json_str.contains("This is a test response"));
    assert!(json_str.contains("gpt-4"));
    assert!(json_str.contains("stop"));

    // Test JSON deserialization
    let deserialized: Response = serde_json::from_str(&json_str)?;
    assert_eq!(deserialized.content, "This is a test response.");
    assert_eq!(deserialized.usage.prompt_tokens, 15);
    assert_eq!(deserialized.usage.completion_tokens, 25);
    assert_eq!(deserialized.usage.total_tokens, 40);
    assert_eq!(deserialized.model, "gpt-4");

    Ok(())
}

#[test]
fn test_usage_calculations() -> Result<()> {
    let usage = Usage {
        prompt_tokens: 100,
        completion_tokens: 50,
        total_tokens: 150,
    };

    // Verify that total_tokens matches the sum
    assert_eq!(usage.total_tokens, usage.prompt_tokens + usage.completion_tokens);

    // Test edge cases
    let zero_usage = Usage {
        prompt_tokens: 0,
        completion_tokens: 0,
        total_tokens: 0,
    };
    assert_eq!(zero_usage.total_tokens, 0);

    let large_usage = Usage {
        prompt_tokens: 1000000,
        completion_tokens: 500000,
        total_tokens: 1500000,
    };
    assert_eq!(large_usage.total_tokens, 1500000);

    Ok(())
}

#[test]
fn test_cost_calculations() -> Result<()> {
    let cost = Cost {
        prompt_cost: 0.01,
        completion_cost: 0.02,
        total_cost: 0.03,
    };

    // Verify that total_cost matches the sum
    assert_eq!(cost.total_cost, cost.prompt_cost + cost.completion_cost);

    // Test edge cases
    let zero_cost = Cost {
        prompt_cost: 0.0,
        completion_cost: 0.0,
        total_cost: 0.0,
    };
    assert_eq!(zero_cost.total_cost, 0.0);

    let high_precision_cost = Cost {
        prompt_cost: 0.000123,
        completion_cost: 0.000456,
        total_cost: 0.000579,
    };
    assert!((high_precision_cost.total_cost - (high_precision_cost.prompt_cost + high_precision_cost.completion_cost)).abs() < 0.000001);

    Ok(())
}

#[test]
fn test_extracted_content() -> Result<()> {
    let extracted = ExtractedContent {
        main_content: "This is extracted text content.".to_string(),
        sentiment: Some("positive".to_string()),
        clusters: Some(vec!["technology".to_string()]),
        themes: Some(vec!["innovation".to_string()]),
        keywords: Some(vec!["technology".to_string(), "content".to_string()]),
    };

    assert_eq!(extracted.main_content, "This is extracted text content.");
    assert_eq!(extracted.sentiment, Some("positive".to_string()));
    assert!(extracted.clusters.is_some());
    assert!(extracted.themes.is_some());
    assert!(extracted.keywords.is_some());

    // Test with minimal content
    let minimal = ExtractedContent {
        main_content: "Minimal content".to_string(),
        sentiment: None,
        clusters: None,
        themes: None,
        keywords: None,
    };
    assert_eq!(minimal.main_content, "Minimal content");
    assert!(minimal.sentiment.is_none());

    Ok(())
}

#[test]
fn test_upsert_request() -> Result<()> {
    let upsert_request = UpsertRequest {
        input: "What is the capital of France?".to_string(),
        output: "The capital of France is Paris.".to_string(),
        metadata: vec!["geography".to_string(), "europe".to_string()],
    };

    assert_eq!(upsert_request.input, "What is the capital of France?");
    assert_eq!(upsert_request.output, "The capital of France is Paris.");
    assert_eq!(upsert_request.metadata.len(), 2);
    assert!(upsert_request.metadata.contains(&"geography".to_string()));
    assert!(upsert_request.metadata.contains(&"europe".to_string()));

    Ok(())
}

#[test]
fn test_upsert_response() -> Result<()> {
    let upsert_response = UpsertResponse {
        processed_files: vec!["conversation_1.json".to_string(), "conversation_2.json".to_string()],
        errors: vec![],
    };

    assert_eq!(upsert_response.processed_files.len(), 2);
    assert!(upsert_response.processed_files.contains(&"conversation_1.json".to_string()));
    assert!(upsert_response.processed_files.contains(&"conversation_2.json".to_string()));
    assert!(upsert_response.errors.is_empty());

    // Test failure case
    let failure_response = UpsertResponse {
        processed_files: vec!["conversation_1.json".to_string()],
        errors: vec!["Failed to process conversation_2.json".to_string()],
    };
    assert_eq!(failure_response.processed_files.len(), 1);
    assert_eq!(failure_response.errors.len(), 1);
    assert!(failure_response.errors.contains(&"Failed to process conversation_2.json".to_string()));

    Ok(())
}

#[test]
fn test_type_edge_cases() -> Result<()> {
    // Test empty strings
    let empty_request = Request {
        flowname: "".to_string(),
        payload: "".to_string(),
    };
    assert_eq!(empty_request.flowname, "");
    assert_eq!(empty_request.payload, "");

    // Test very long strings
    let long_content = "a".repeat(10000);
    let long_request = Request {
        flowname: "long_test".to_string(),
        payload: long_content.clone(),
    };
    assert_eq!(long_request.payload.len(), 10000);

    // Test special characters
    let special_request = Request {
        flowname: "special_chars".to_string(),
        payload: "Hello 🌍! Special chars: @#$%^&*()".to_string(),
    };
    assert!(special_request.payload.contains("🌍"));
    assert!(special_request.payload.contains("@#$%^&*()"));

    Ok(())
}

#[test]
fn test_response_optional_fields() -> Result<()> {
    let usage = Usage {
        prompt_tokens: 5,
        completion_tokens: 10,
        total_tokens: 15,
    };

    let cost = Cost {
        prompt_cost: 0.001,
        completion_cost: 0.002,
        total_cost: 0.003,
    };

    // Test response with minimal fields
    let minimal_response = Response {
        content: "Minimal response".to_string(),
        usage,
        cost,
        model: "gpt-3.5-turbo".to_string(),
        finish_reason: None,
    };

    assert_eq!(minimal_response.content, "Minimal response");
    assert_eq!(minimal_response.model, "gpt-3.5-turbo");
    assert!(minimal_response.finish_reason.is_none());

    // Test response with all optional fields
    let full_response = Response {
        content: "Full response".to_string(),
        usage: Usage {
            prompt_tokens: 10,
            completion_tokens: 20,
            total_tokens: 30,
        },
        cost: Cost {
            prompt_cost: 0.002,
            completion_cost: 0.004,
            total_cost: 0.006,
        },
        model: "gpt-4-turbo".to_string(),
        finish_reason: Some("length".to_string()),
    };

    assert_eq!(full_response.content, "Full response");
    assert_eq!(full_response.model, "gpt-4-turbo");
    assert_eq!(full_response.finish_reason, Some("length".to_string()));

    Ok(())
}
