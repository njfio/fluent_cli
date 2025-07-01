use crate::modular_pipeline_executor::{StepExecutor, StepResult, ExecutionContext, PipelineStep};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use tokio::process::Command;

/// Concrete step executors for common pipeline operations

/// Command step executor for running shell commands
pub struct CommandStepExecutor;

#[async_trait]
impl StepExecutor for CommandStepExecutor {
    async fn execute(&self, step: &PipelineStep, context: &mut ExecutionContext) -> Result<StepResult> {
        let command = step.config.get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Command step requires 'command' config"))?;

        let shell = step.config.get("shell")
            .and_then(|v| v.as_str())
            .unwrap_or("sh");

        let shell_arg = if shell == "cmd" { "/C" } else { "-c" };

        // Expand variables in command
        let expanded_command = self.expand_variables(command, &context.variables)?;

        // Execute command
        let output = Command::new(shell)
            .arg(shell_arg)
            .arg(&expanded_command)
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !output.status.success() {
            return Err(anyhow!("Command failed with exit code {:?}: {}", output.status.code(), stderr));
        }

        let mut variables = HashMap::new();
        if let Some(save_output) = step.config.get("save_output").and_then(|v| v.as_str()) {
            variables.insert(save_output.to_string(), stdout.trim().to_string());
        }

        Ok(StepResult {
            output: Some(stdout),
            variables,
            metadata: [("exit_code".to_string(), output.status.code().unwrap_or(0).to_string())].into_iter().collect(),
        })
    }

    fn supported_types(&self) -> Vec<String> {
        vec!["command".to_string(), "shell".to_string()]
    }

    fn validate_config(&self, step: &PipelineStep) -> Result<()> {
        if !step.config.contains_key("command") {
            return Err(anyhow!("Command step requires 'command' config"));
        }
        Ok(())
    }
}

impl CommandStepExecutor {
    fn expand_variables(&self, template: &str, variables: &HashMap<String, String>) -> Result<String> {
        let mut result = template.to_string();
        for (key, value) in variables {
            let placeholder = format!("${{{}}}", key);
            result = result.replace(&placeholder, value);
        }
        Ok(result)
    }
}

/// HTTP request step executor
pub struct HttpStepExecutor {
    client: reqwest::Client,
}

impl HttpStepExecutor {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl StepExecutor for HttpStepExecutor {
    async fn execute(&self, step: &PipelineStep, context: &mut ExecutionContext) -> Result<StepResult> {
        let url = step.config.get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("HTTP step requires 'url' config"))?;

        let method = step.config.get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("GET");

        let headers = step.config.get("headers")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect::<HashMap<String, String>>()
            })
            .unwrap_or_default();

        let body = step.config.get("body")
            .and_then(|v| v.as_str());

        // Expand variables in URL and body
        let expanded_url = self.expand_variables(url, &context.variables)?;
        let expanded_body = if let Some(body) = body {
            Some(self.expand_variables(body, &context.variables)?)
        } else {
            None
        };

        // Build request
        let mut request = match method.to_uppercase().as_str() {
            "GET" => self.client.get(&expanded_url),
            "POST" => self.client.post(&expanded_url),
            "PUT" => self.client.put(&expanded_url),
            "DELETE" => self.client.delete(&expanded_url),
            "PATCH" => self.client.patch(&expanded_url),
            _ => return Err(anyhow!("Unsupported HTTP method: {}", method)),
        };

        // Add headers
        for (key, value) in headers {
            request = request.header(&key, &value);
        }

        // Add body if present
        if let Some(body) = expanded_body {
            request = request.body(body);
        }

        // Execute request
        let response = request.send().await?;
        let status = response.status();
        let response_text = response.text().await?;

        let mut variables = HashMap::new();
        if let Some(save_output) = step.config.get("save_output").and_then(|v| v.as_str()) {
            variables.insert(save_output.to_string(), response_text.clone());
        }

        if let Some(save_status) = step.config.get("save_status").and_then(|v| v.as_str()) {
            variables.insert(save_status.to_string(), status.as_u16().to_string());
        }

        if !status.is_success() {
            return Err(anyhow!("HTTP request failed with status {}: {}", status, response_text));
        }

        Ok(StepResult {
            output: Some(response_text),
            variables,
            metadata: [
                ("status_code".to_string(), status.as_u16().to_string()),
                ("method".to_string(), method.to_string()),
                ("url".to_string(), expanded_url),
            ].into_iter().collect(),
        })
    }

    fn supported_types(&self) -> Vec<String> {
        vec!["http".to_string(), "api".to_string(), "request".to_string()]
    }

    fn validate_config(&self, step: &PipelineStep) -> Result<()> {
        if !step.config.contains_key("url") {
            return Err(anyhow!("HTTP step requires 'url' config"));
        }
        Ok(())
    }
}

impl HttpStepExecutor {
    fn expand_variables(&self, template: &str, variables: &HashMap<String, String>) -> Result<String> {
        let mut result = template.to_string();
        for (key, value) in variables {
            let placeholder = format!("${{{}}}", key);
            result = result.replace(&placeholder, value);
        }
        Ok(result)
    }
}

/// File operation step executor
pub struct FileStepExecutor;

#[async_trait]
impl StepExecutor for FileStepExecutor {
    async fn execute(&self, step: &PipelineStep, context: &mut ExecutionContext) -> Result<StepResult> {
        let operation = step.config.get("operation")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("File step requires 'operation' config"))?;

        match operation {
            "read" => self.read_file(step, context).await,
            "write" => self.write_file(step, context).await,
            "copy" => self.copy_file(step, context).await,
            "delete" => self.delete_file(step, context).await,
            "exists" => self.check_file_exists(step, context).await,
            _ => Err(anyhow!("Unsupported file operation: {}", operation)),
        }
    }

    fn supported_types(&self) -> Vec<String> {
        vec!["file".to_string(), "fs".to_string()]
    }

    fn validate_config(&self, step: &PipelineStep) -> Result<()> {
        if !step.config.contains_key("operation") {
            return Err(anyhow!("File step requires 'operation' config"));
        }
        Ok(())
    }
}

impl FileStepExecutor {
    async fn read_file(&self, step: &PipelineStep, context: &ExecutionContext) -> Result<StepResult> {
        let path = step.config.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Read operation requires 'path' config"))?;

        let expanded_path = self.expand_variables(path, &context.variables)?;
        let content = tokio::fs::read_to_string(&expanded_path).await?;

        let mut variables = HashMap::new();
        if let Some(save_output) = step.config.get("save_output").and_then(|v| v.as_str()) {
            variables.insert(save_output.to_string(), content.clone());
        }

        Ok(StepResult {
            output: Some(content),
            variables,
            metadata: [("file_path".to_string(), expanded_path)].into_iter().collect(),
        })
    }

    async fn write_file(&self, step: &PipelineStep, context: &ExecutionContext) -> Result<StepResult> {
        let path = step.config.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Write operation requires 'path' config"))?;

        let content = step.config.get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Write operation requires 'content' config"))?;

        let expanded_path = self.expand_variables(path, &context.variables)?;
        let expanded_content = self.expand_variables(content, &context.variables)?;

        tokio::fs::write(&expanded_path, &expanded_content).await?;

        Ok(StepResult {
            output: Some(format!("Written {} bytes to {}", expanded_content.len(), expanded_path)),
            variables: HashMap::new(),
            metadata: [
                ("file_path".to_string(), expanded_path),
                ("bytes_written".to_string(), expanded_content.len().to_string()),
            ].into_iter().collect(),
        })
    }

    async fn copy_file(&self, step: &PipelineStep, context: &ExecutionContext) -> Result<StepResult> {
        let source = step.config.get("source")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Copy operation requires 'source' config"))?;

        let destination = step.config.get("destination")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Copy operation requires 'destination' config"))?;

        let expanded_source = self.expand_variables(source, &context.variables)?;
        let expanded_destination = self.expand_variables(destination, &context.variables)?;

        tokio::fs::copy(&expanded_source, &expanded_destination).await?;

        Ok(StepResult {
            output: Some(format!("Copied {} to {}", expanded_source, expanded_destination)),
            variables: HashMap::new(),
            metadata: [
                ("source_path".to_string(), expanded_source),
                ("destination_path".to_string(), expanded_destination),
            ].into_iter().collect(),
        })
    }

    async fn delete_file(&self, step: &PipelineStep, context: &ExecutionContext) -> Result<StepResult> {
        let path = step.config.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Delete operation requires 'path' config"))?;

        let expanded_path = self.expand_variables(path, &context.variables)?;
        tokio::fs::remove_file(&expanded_path).await?;

        Ok(StepResult {
            output: Some(format!("Deleted {}", expanded_path)),
            variables: HashMap::new(),
            metadata: [("deleted_path".to_string(), expanded_path)].into_iter().collect(),
        })
    }

    async fn check_file_exists(&self, step: &PipelineStep, context: &ExecutionContext) -> Result<StepResult> {
        let path = step.config.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Exists operation requires 'path' config"))?;

        let expanded_path = self.expand_variables(path, &context.variables)?;
        let exists = tokio::fs::metadata(&expanded_path).await.is_ok();

        let mut variables = HashMap::new();
        if let Some(save_output) = step.config.get("save_output").and_then(|v| v.as_str()) {
            variables.insert(save_output.to_string(), exists.to_string());
        }

        Ok(StepResult {
            output: Some(exists.to_string()),
            variables,
            metadata: [
                ("file_path".to_string(), expanded_path),
                ("exists".to_string(), exists.to_string()),
            ].into_iter().collect(),
        })
    }

    fn expand_variables(&self, template: &str, variables: &HashMap<String, String>) -> Result<String> {
        let mut result = template.to_string();
        for (key, value) in variables {
            let placeholder = format!("${{{}}}", key);
            result = result.replace(&placeholder, value);
        }
        Ok(result)
    }
}

/// Condition step executor for conditional logic
pub struct ConditionStepExecutor;

#[async_trait]
impl StepExecutor for ConditionStepExecutor {
    async fn execute(&self, step: &PipelineStep, context: &mut ExecutionContext) -> Result<StepResult> {
        let condition = step.config.get("condition")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Condition step requires 'condition' config"))?;

        let if_true = step.config.get("if_true")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let if_false = step.config.get("if_false")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // Simple condition evaluation (can be enhanced with a proper expression evaluator)
        let result = self.evaluate_condition(condition, &context.variables)?;
        let output = if result { if_true } else { if_false };

        let mut variables = HashMap::new();
        if let Some(save_output) = step.config.get("save_output").and_then(|v| v.as_str()) {
            variables.insert(save_output.to_string(), output.to_string());
        }

        Ok(StepResult {
            output: Some(output.to_string()),
            variables,
            metadata: [
                ("condition_result".to_string(), result.to_string()),
                ("condition".to_string(), condition.to_string()),
            ].into_iter().collect(),
        })
    }

    fn supported_types(&self) -> Vec<String> {
        vec!["condition".to_string(), "if".to_string()]
    }

    fn validate_config(&self, step: &PipelineStep) -> Result<()> {
        if !step.config.contains_key("condition") {
            return Err(anyhow!("Condition step requires 'condition' config"));
        }
        Ok(())
    }
}

impl ConditionStepExecutor {
    fn evaluate_condition(&self, condition: &str, variables: &HashMap<String, String>) -> Result<bool> {
        // Expand variables first
        let mut expanded = condition.to_string();
        for (key, value) in variables {
            let placeholder = format!("${{{}}}", key);
            expanded = expanded.replace(&placeholder, value);
        }

        // Simple condition evaluation
        match expanded.trim() {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => {
                // Try to evaluate simple comparisons
                if expanded.contains("==") {
                    let parts: Vec<&str> = expanded.split("==").collect();
                    if parts.len() == 2 {
                        return Ok(parts[0].trim() == parts[1].trim());
                    }
                }
                if expanded.contains("!=") {
                    let parts: Vec<&str> = expanded.split("!=").collect();
                    if parts.len() == 2 {
                        return Ok(parts[0].trim() != parts[1].trim());
                    }
                }
                
                // Default to false for unknown conditions
                Ok(false)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modular_pipeline_executor::ExecutionContext;
    use std::time::SystemTime;

    #[tokio::test]
    async fn test_command_step_executor() {
        let executor = CommandStepExecutor;
        let mut step = PipelineStep {
            name: "test".to_string(),
            step_type: "command".to_string(),
            config: [
                ("command".to_string(), serde_json::json!("echo hello")),
                ("save_output".to_string(), serde_json::json!("result")),
            ].into_iter().collect(),
            timeout: None,
            retry_config: None,
            depends_on: Vec::new(),
            condition: None,
            parallel_group: None,
        };

        let mut context = ExecutionContext {
            run_id: "test".to_string(),
            pipeline_name: "test".to_string(),
            current_step: 0,
            variables: HashMap::new(),
            metadata: HashMap::new(),
            start_time: SystemTime::now(),
            step_history: Vec::new(),
        };

        let result = executor.execute(&step, &mut context).await.unwrap();
        assert!(result.output.is_some());
        assert!(result.variables.contains_key("result"));
    }

    #[test]
    fn test_condition_evaluation() {
        let executor = ConditionStepExecutor;
        let variables = [("status".to_string(), "success".to_string())].into_iter().collect();
        
        assert!(executor.evaluate_condition("true", &variables).unwrap());
        assert!(!executor.evaluate_condition("false", &variables).unwrap());
        assert!(executor.evaluate_condition("${status} == success", &variables).unwrap());
        assert!(!executor.evaluate_condition("${status} == failure", &variables).unwrap());
    }
}
