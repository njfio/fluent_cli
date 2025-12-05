use anyhow::Result;
use fluent_agent::Agent;
use fluent_core::neo4j_client::Neo4jClient;
use fluent_core::types::{
    Cost, ExtractedContent, Request, Response, UpsertRequest, UpsertResponse, Usage,
};
use std::future::Future;
use std::path::Path;
use std::sync::Arc;

struct NoopEngine;

impl fluent_core::traits::Engine for NoopEngine {
    fn execute<'a>(
        &'a self,
        _request: &'a Request,
    ) -> Box<dyn Future<Output = Result<Response>> + Send + 'a> {
        Box::new(async move {
            Ok(Response {
                content: String::new(),
                usage: Usage {
                    prompt_tokens: 0,
                    completion_tokens: 0,
                    total_tokens: 0,
                },
                model: "noop".to_string(),
                finish_reason: Some("stop".to_string()),
                cost: Cost {
                    prompt_cost: 0.0,
                    completion_cost: 0.0,
                    total_cost: 0.0,
                },
            })
        })
    }

    fn upsert<'a>(
        &'a self,
        _request: &'a UpsertRequest,
    ) -> Box<dyn Future<Output = Result<UpsertResponse>> + Send + 'a> {
        Box::new(async move {
            Ok(UpsertResponse {
                processed_files: vec![],
                errors: vec![],
            })
        })
    }

    fn get_neo4j_client(&self) -> Option<&Arc<Neo4jClient>> {
        None
    }

    fn get_session_id(&self) -> Option<String> {
        None
    }

    fn extract_content(&self, _value: &serde_json::Value) -> Option<ExtractedContent> {
        None
    }

    fn upload_file<'a>(
        &'a self,
        _file_path: &'a Path,
    ) -> Box<dyn Future<Output = Result<String>> + Send + 'a> {
        Box::new(async move { Ok(String::new()) })
    }

    fn process_request_with_file<'a>(
        &'a self,
        _request: &'a Request,
        _file_path: &'a Path,
    ) -> Box<dyn Future<Output = Result<Response>> + Send + 'a> {
        Box::new(async move {
            Ok(Response {
                content: String::new(),
                usage: Usage {
                    prompt_tokens: 0,
                    completion_tokens: 0,
                    total_tokens: 0,
                },
                model: "noop".to_string(),
                finish_reason: Some("stop".to_string()),
                cost: Cost {
                    prompt_cost: 0.0,
                    completion_cost: 0.0,
                    total_cost: 0.0,
                },
            })
        })
    }
}

#[tokio::test]
async fn denies_disallowed_command_by_default() {
    let agent = Agent::new(Box::new(NoopEngine));
    let err = agent
        .run_command("notallowedcmd", &[])
        .await
        .err()
        .expect("should error");
    assert!(err.to_string().contains("not in allowed list"));
}

#[tokio::test]
async fn denies_dangerous_metacharacters_in_args() {
    std::env::set_var("FLUENT_ALLOWED_COMMANDS", "echo");
    let agent = Agent::new(Box::new(NoopEngine));
    let err = agent
        .run_command("echo", &["hello; rm -rf /"])
        .await
        .err()
        .expect("should error on dangerous arg");
    assert!(err.to_string().contains("dangerous pattern"));
}

#[tokio::test]
async fn allows_safe_command_when_whitelisted() {
    std::env::set_var("FLUENT_ALLOWED_COMMANDS", "echo");
    let agent = Agent::new(Box::new(NoopEngine));
    let out = agent
        .run_command("echo", &["hello-world"])
        .await
        .expect("echo should succeed");
    assert!(out.contains("hello-world"));
}
