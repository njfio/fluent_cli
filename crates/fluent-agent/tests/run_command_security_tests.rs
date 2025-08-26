use anyhow::Result;
use fluent_agent::Agent;
use fluent_core::types::Request;

struct NoopEngine;
#[async_trait::async_trait]
impl fluent_core::traits::Engine for NoopEngine {
    async fn execute(&self, _request: &Request) -> Result<fluent_core::types::Response> {
        Ok(fluent_core::types::Response { content: String::new() })
    }
    async fn upload_file(&self, _path: &std::path::Path) -> Result<String> { Ok(String::new()) }
}

#[tokio::test]
async fn denies_disallowed_command_by_default() {
    let agent = Agent::new(Box::new(NoopEngine));
    let err = agent.run_command("notallowedcmd", &[]).await.err().expect("should error");
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
