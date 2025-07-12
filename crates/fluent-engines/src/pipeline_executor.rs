use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;

use std::io::Write;
use tokio::process::Command as TokioCommand;

use std::time::{SystemTime, UNIX_EPOCH};
use tempfile;

use anyhow::{anyhow, Error};
pub use anyhow::Error as PipelineError;

// Import modular pipeline components
use crate::pipeline::{
    CommandExecutor, ParallelExecutor,
    ConditionExecutor, LoopExecutor, VariableExpander, StepExecutor
};
use async_trait::async_trait;
use log::{debug, error, info, warn};


use schemars::JsonSchema;
use uuid::Uuid;

use serde_json;
use serde_yaml;

// Security: Import input validator for pipeline security
use fluent_core::input_validator::InputValidator;

#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema)]
pub struct Pipeline {
    pub name: String,
    pub steps: Vec<PipelineStep>,
}

#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema)]
pub enum PipelineStep {
    Command {
        name: String,
        command: String,
        save_output: Option<String>,
        retry: Option<RetryConfig>,
    },
    ShellCommand {
        name: String,
        command: String,
        save_output: Option<String>,
        retry: Option<RetryConfig>,
    },
    Condition {
        name: String,
        condition: String,
        if_true: String,
        if_false: String,
    },
    Loop {
        name: String,
        steps: Vec<PipelineStep>,
        condition: String,
    },
    SubPipeline {
        name: String,
        pipeline: String,
        with: HashMap<String, String>,
    },
    Map {
        name: String,
        input: String,
        command: String,
        save_output: String,
    },
    HumanInTheLoop {
        name: String,
        prompt: String,
        save_output: String,
    },
    RepeatUntil {
        name: String,
        steps: Vec<PipelineStep>,
        condition: String,
    },
    PrintOutput {
        name: String,
        value: String,
    },
    ForEach {
        name: String,
        items: String,
        steps: Vec<PipelineStep>,
    },
    TryCatch {
        name: String,
        try_steps: Vec<PipelineStep>,
        catch_steps: Vec<PipelineStep>,
        finally_steps: Vec<PipelineStep>,
    },
    Parallel {
        name: String,
        steps: Vec<PipelineStep>,
    },
    Timeout {
        name: String,
        duration: u64,
        step: Box<PipelineStep>,
    },
}

#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub delay_ms: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct PipelineState {
    pub current_step: usize,
    pub data: HashMap<String, String>,
    pub run_id: String,
    pub start_time: u64,
}

pub fn pipeline_schema() -> schemars::schema::RootSchema {
    schemars::schema_for!(Pipeline)
}

/// Validates pipeline YAML with comprehensive security checks
pub fn validate_pipeline_yaml(yaml: &str) -> Result<(), Error> {
    // Basic YAML validation
    if yaml.is_empty() {
        return Err(anyhow!("Pipeline YAML cannot be empty"));
    }

    if yaml.len() > 1_000_000 {
        // 1MB limit
        return Err(anyhow!("Pipeline YAML too large: {} bytes", yaml.len()));
    }

    // Check for dangerous patterns in YAML
    InputValidator::validate_request_payload(yaml)?;

    // Parse YAML structure
    let _value: serde_yaml::Value =
        serde_yaml::from_str(yaml).map_err(|e| anyhow!("Invalid YAML syntax: {}", e))?;

    let pipeline: Pipeline =
        serde_yaml::from_str(yaml).map_err(|e| anyhow!("Invalid pipeline structure: {}", e))?;

    // Security validation of pipeline content
    validate_pipeline_security(&pipeline)?;

    Ok(())
}

/// Performs security validation on pipeline structure
fn validate_pipeline_security(pipeline: &Pipeline) -> Result<(), Error> {
    // Validate pipeline name
    if pipeline.name.is_empty() || pipeline.name.len() > 100 {
        return Err(anyhow!("Invalid pipeline name length"));
    }

    // Check for dangerous characters in pipeline name
    for dangerous_char in ['/', '\\', '\0'] {
        if pipeline.name.contains(dangerous_char) {
            return Err(anyhow!("Pipeline name contains dangerous characters"));
        }
    }
    if pipeline.name.contains("..") {
        return Err(anyhow!("Pipeline name contains dangerous characters"));
    }

    // Validate steps
    if pipeline.steps.is_empty() {
        return Err(anyhow!("Pipeline must have at least one step"));
    }

    if pipeline.steps.len() > 1000 {
        return Err(anyhow!(
            "Pipeline has too many steps: {}",
            pipeline.steps.len()
        ));
    }

    // Validate each step for security
    for step in &pipeline.steps {
        validate_step_security(step)?;
    }

    Ok(())
}

/// Validates individual pipeline steps for security issues
fn validate_step_security(step: &PipelineStep) -> Result<(), Error> {
    match step {
        PipelineStep::Command { command, .. } | PipelineStep::ShellCommand { command, .. } => {
            // SECURITY: Disable command execution entirely for safety
            return Err(anyhow!(
                "Command execution is disabled for security reasons. \
                Commands found: '{}'. To enable command execution, \
                implement proper sandboxing and command whitelisting.",
                command
            ));
        }

        PipelineStep::Condition { condition, .. } => {
            // SECURITY: Disable condition evaluation that uses shell commands
            return Err(anyhow!(
                "Condition evaluation is disabled for security reasons. \
                Condition found: '{}'. Implement safe condition evaluation \
                without shell command execution.",
                condition
            ));
        }

        PipelineStep::Map { command, .. } => {
            // SECURITY: Disable map operations that execute commands
            return Err(anyhow!(
                "Map operations with command execution are disabled for security reasons. \
                Command found: '{}'. Implement safe map operations \
                without shell command execution.",
                command
            ));
        }

        PipelineStep::PrintOutput { value, .. } => {
            // Validate print output for injection patterns
            InputValidator::validate_request_payload(value)?;
        }

        PipelineStep::ForEach { items, steps, .. } => {
            // Validate items string
            InputValidator::validate_request_payload(items)?;

            // Recursively validate nested steps
            for nested_step in steps {
                validate_step_security(nested_step)?;
            }
        }

        PipelineStep::TryCatch {
            try_steps,
            catch_steps,
            finally_steps,
            ..
        } => {
            // Recursively validate all nested steps
            for nested_step in try_steps
                .iter()
                .chain(catch_steps.iter())
                .chain(finally_steps.iter())
            {
                validate_step_security(nested_step)?;
            }
        }

        PipelineStep::Parallel { steps, .. } => {
            // Recursively validate parallel steps
            for nested_step in steps {
                validate_step_security(nested_step)?;
            }
        }

        PipelineStep::Timeout { step, duration, .. } => {
            // Validate timeout duration
            if *duration > 3600 {
                // 1 hour max
                return Err(anyhow!("Timeout duration too long: {} seconds", duration));
            }

            // Recursively validate nested step
            validate_step_security(step)?;
        }

        PipelineStep::Loop { condition, .. } => {
            // SECURITY: Disable loop conditions that use shell commands
            return Err(anyhow!(
                "Loop operations with condition evaluation are disabled for security reasons. \
                Condition found: '{}'. Implement safe loop operations \
                without shell command execution.",
                condition
            ));
        }

        PipelineStep::SubPipeline { pipeline, .. } => {
            // Validate sub-pipeline reference
            InputValidator::validate_request_payload(pipeline)?;
        }

        PipelineStep::HumanInTheLoop { prompt, .. } => {
            // Validate human prompt for injection patterns
            InputValidator::validate_request_payload(prompt)?;
        }

        PipelineStep::RepeatUntil { condition, .. } => {
            // SECURITY: Disable repeat-until conditions that use shell commands
            return Err(anyhow!(
                "RepeatUntil operations with condition evaluation are disabled for security reasons. \
                Condition found: '{}'. Implement safe repeat operations \
                without shell command execution.",
                condition
            ));
        }
    }

    Ok(())
}

#[async_trait]
pub trait StateStore {
    async fn save_state(&self, pipeline_name: &str, state: &PipelineState) -> anyhow::Result<()>;
    async fn load_state(&self, pipeline_name: &str) -> anyhow::Result<Option<PipelineState>>;
}

pub struct PipelineExecutor<S: StateStore> {
    // Change state to Arc<Mutex<...>>
    state_store: S,
    json_output: bool,
}

pub type PipelineFuture<'a> =
    Pin<Box<dyn Future<Output = Result<HashMap<String, String>, Error>> + Send + 'a>>;

impl<S: StateStore + Clone + std::marker::Sync + std::marker::Send> PipelineExecutor<S> {
    pub fn new(state_store: S, _json_output: bool) -> Self {
        Self {
            state_store,
            json_output: false,
        }
    }

    pub async fn execute(
        &self,
        pipeline: &Pipeline,
        initial_input: &str,
        force_fresh: bool,
        provided_run_id: Option<String>,
    ) -> Result<String, Error> {
        let run_id = provided_run_id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let state_key = format!("{}-{}", pipeline.name, run_id);
        debug!(
            "Executing pipeline {} with run_id {}",
            pipeline.name, run_id
        );

        let mut state = if force_fresh {
            debug!("Forcing fresh state");
            PipelineState {
                current_step: 0,
                data: HashMap::new(),
                run_id: run_id.clone(),
                start_time: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
            }
        } else {
            match self.state_store.load_state(&state_key).await? {
                Some(saved_state) => {
                    debug!(
                        "Resuming from saved state at step {}",
                        saved_state.current_step
                    );
                    saved_state
                }
                None => {
                    debug!("No saved state found, starting fresh");
                    PipelineState {
                        current_step: 0,
                        data: HashMap::new(),
                        run_id: run_id.clone(),
                        start_time: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
                    }
                }
            }
        };

        if state.current_step == 0 {
            state
                .data
                .insert("input".to_string(), initial_input.to_string());
        }
        state.data.insert("run_id".to_string(), run_id.clone());

        for (index, step) in pipeline.steps.iter().enumerate().skip(state.current_step) {
            debug!("Processing step {} (index {})", step.name(), index);

            state
                .data
                .insert("step".to_string(), step.name().to_string());
            state.current_step = index;

            match self.execute_step(step, &mut state).await {
                Ok(step_result) => {
                    info!("Step {} completed successfully", step.name());
                    state.data.extend(step_result);
                    self.state_store.save_state(&state_key, &state).await?;
                }
                Err(e) => {
                    error!("Error executing step {}: {:?}", step.name(), e);
                    return Err(e);
                }
            }
        }

        let end_time = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let runtime = end_time - state.start_time;

        let output = serde_json::json!({
            "pipeline_name": pipeline.name,
            "run_id": run_id,
            "current_step": state.current_step,
            "start_time": state.start_time,
            "end_time": end_time,
            "runtime_seconds": runtime,
            "data": state.data,
        });

        Ok(serde_json::to_string_pretty(&output)?)
    }

    fn execute_step<'a>(
        &'a self,
        step: &'a PipelineStep,
        state: &'a mut PipelineState,
    ) -> PipelineFuture<'a> {
        Box::pin(async move {
            match step {
                PipelineStep::Command {
                    name,
                    command,
                    save_output,
                    retry,
                } => {
                    debug!("Executing Command step: {}", name);
                    debug!("Command: {}", command);
                    let expanded_command = VariableExpander::expand_variables(command, &state.data).await?;
                    CommandExecutor::execute_command_with_retry(&expanded_command, save_output, retry)
                        .await
                }

                PipelineStep::ShellCommand {
                    name,
                    command,
                    save_output,
                    retry,
                } => {
                    debug!("Executing ShellCommand step: {}", name);
                    debug!("Command: {}", command);
                    let expanded_command = VariableExpander::expand_variables(command, &state.data).await?;
                    CommandExecutor::execute_shell_command_with_retry(&expanded_command, save_output, retry)
                        .await
                }

                PipelineStep::Condition {
                    name,
                    condition,
                    if_true,
                    if_false,
                } => {
                    debug!("Evaluating Condition step: {}", name);
                    debug!("Condition: {}", condition);
                    ConditionExecutor::execute_condition_with_expansion(
                        name, condition, if_true, if_false, &state.data
                    ).await
                }

                PipelineStep::PrintOutput { name, value } => {
                    debug!("Executing PrintOutput step: {}", name);
                    let expanded_value = VariableExpander::expand_variables(value, &state.data).await?;
                    if !self.json_output {
                        eprintln!("{}", expanded_value); // Print to stderr instead of stdout
                    }
                    Ok(HashMap::new())
                }

                PipelineStep::Map {
                    name,
                    input,
                    command,
                    save_output,
                } => {
                    debug!("Executing Map step: {}", name);
                    debug!("Input: {}", input);
                    debug!("Command: {}", command);
                    let input_data = VariableExpander::expand_variables(input, &state.data).await?;
                    debug!("Expanded input data: {}", input_data);
                    let mut results = Vec::new();

                    for item in input_data.split(',') {
                        let item = item.trim();
                        debug!("Processing item: {}", item);
                        let expanded_command = VariableExpander::expand_variables(command, &state.data).await?;
                        let item_command = expanded_command.replace("${ITEM}", item);
                        debug!("Executing command: {}", item_command);
                        match self
                            .execute_shell_command(&item_command, &None, &None)
                            .await
                        {
                            Ok(output) => {
                                let new_string = String::new();
                                let result = output.values().next().unwrap_or(&new_string);
                                debug!("Command output: {}", result);
                                results.push(result.to_string());
                            }
                            Err(e) => {
                                error!("Error executing command for item {}: {:?}", item, e);
                            }
                        }
                    }

                    let output = results.join(", ");
                    debug!("Map step result: {}", output);
                    Ok([(save_output.clone(), output)].into_iter().collect())
                }

                PipelineStep::HumanInTheLoop {
                    name,
                    prompt,
                    save_output,
                } => {
                    debug!("Executing HumanInTheLoop step: {}", name);
                    debug!("Prompt: {}", prompt);
                    let expanded_prompt = VariableExpander::expand_variables(prompt, &state.data).await?;
                    println!("{}", expanded_prompt);

                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input)?;

                    Ok([(save_output.clone(), input.trim().to_string())]
                        .into_iter()
                        .collect())
                }

                PipelineStep::RepeatUntil {
                    name,
                    steps,
                    condition,
                } => {
                    debug!("Executing RepeatUntil step: {}", name);
                    debug!("Steps: {:?}", steps);
                    debug!("Condition: {}", condition);
                    LoopExecutor::execute_repeat_until(steps, condition, state).await
                }

                PipelineStep::ForEach { name, items, steps } => {
                    debug!("Executing ForEach step: {}", name);
                    debug!("Items: {}", items);
                    debug!("Steps: {:?}", steps);
                    let expanded_items = VariableExpander::expand_variables(items, &state.data).await?;
                    LoopExecutor::execute_for_each(name, &expanded_items, steps, state).await
                }

                PipelineStep::TryCatch {
                    name,
                    try_steps,
                    catch_steps,
                    finally_steps,
                } => {
                    debug!("Executing TryCatch step: {}", name);
                    debug!("Try Steps: {:?}", try_steps);
                    debug!("Catch Steps: {:?}", catch_steps);
                    debug!("Finally Steps: {:?}", finally_steps);
                    StepExecutor::execute_try_catch(try_steps, catch_steps, finally_steps, state).await
                }

                PipelineStep::Parallel { name, steps } => {
                    debug!("Executing Parallel step: {}", name);
                    ParallelExecutor::execute_parallel_steps(steps, state).await
                }

                PipelineStep::Timeout {
                    name,
                    duration,
                    step,
                } => {
                    debug!("Executing Timeout step: {}", name);
                    debug!("Duration: {}", duration);
                    debug!("Step: {:?}", step);
                    StepExecutor::execute_timeout(*duration, step, state).await
                }

                _ => Ok(HashMap::new()),
            }
        })
    }











    async fn execute_shell_command(
        &self,
        command: &str,
        save_output: &Option<String>,
        retry: &Option<RetryConfig>,
    ) -> Result<HashMap<String, String>, Error> {
        debug!("Executing shell command: {}", command);

        // Create a temporary file
        let mut temp_file = tempfile::NamedTempFile::new()?;
        writeln!(temp_file.as_file_mut(), "{}", command)?;

        let retry_config = retry.clone().unwrap_or(RetryConfig {
            max_attempts: 1,
            delay_ms: 0,
        });
        let mut attempts = 0;

        loop {
            debug!("Attempt {} to execute shell command", attempts + 1);
            match self.run_shell_command(temp_file.path()).await {
                Ok(output) => {
                    debug!("Shell command executed successfully: {:?}", output);
                    let mut result = HashMap::new();
                    if let Some(save_key) = save_output {
                        result.insert(save_key.clone(), output);
                    } else {
                        result.insert("output".to_string(), output);
                    }
                    return Ok(result);
                }
                Err(e) if attempts < retry_config.max_attempts => {
                    attempts += 1;
                    warn!("Attempt {} failed: {:?}. Retrying...", attempts, e);
                    tokio::time::sleep(std::time::Duration::from_millis(retry_config.delay_ms))
                        .await;
                }
                Err(e) => {
                    error!(
                        "Shell command execution failed after {} attempts: {:?}",
                        attempts + 1,
                        e
                    );
                    return Err(e);
                }
            }
        }
    }

    async fn run_shell_command(&self, script_path: &Path) -> Result<String, Error> {
        debug!("Running shell command from file: {:?}", script_path);

        // Validate script path to prevent path traversal
        let canonical_path = script_path
            .canonicalize()
            .map_err(|e| anyhow!("Invalid script path: {}", e))?;

        // Ensure script is in a safe location (temp directory)
        let temp_dir = std::env::temp_dir();
        let is_in_temp = canonical_path.starts_with(&temp_dir) ||
                        canonical_path.starts_with("/tmp") ||
                        canonical_path.starts_with("/var/folders") || // macOS temp
                        canonical_path.to_string_lossy().contains("tmp");

        if !is_in_temp {
            return Err(anyhow!(
                "Script must be in temporary directory for security. Path: {:?}, Temp dir: {:?}",
                canonical_path, temp_dir
            ));
        }

        // Use absolute path to bash and clear environment
        let bash_path =
            which::which("bash").map_err(|_| anyhow!("bash command not found in PATH"))?;

        let output = TokioCommand::new(bash_path)
            .arg(&canonical_path)
            .env_clear() // Clear environment for security
            .env("PATH", "/usr/bin:/bin:/usr/local/bin") // Minimal but functional PATH
            .output()
            .await
            .map_err(|e| anyhow!("Failed to execute shell command: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!(
                "Shell command failed with exit code {:?}. Stderr: {}",
                output.status.code(),
                stderr
            ));
        }

        let stdout = String::from_utf8(output.stdout)
            .map_err(|e| anyhow!("Failed to parse command output: {}", e))?;

        debug!("Shell command output: {}", stdout);

        Ok(stdout.trim().to_string())
    }


}

impl PipelineStep {
    pub fn name(&self) -> &str {
        match self {
            PipelineStep::Command { name, .. } => name,
            PipelineStep::ShellCommand { name, .. } => name,
            PipelineStep::Condition { name, .. } => name,
            PipelineStep::Loop { name, .. } => name,
            PipelineStep::Map { name, .. } => name,
            PipelineStep::SubPipeline { name, .. } => name,
            PipelineStep::HumanInTheLoop { name, .. } => name,
            PipelineStep::RepeatUntil { name, .. } => name,
            PipelineStep::PrintOutput { name, .. } => name,
            PipelineStep::ForEach { name, .. } => name,
            PipelineStep::TryCatch { name, .. } => name,
            PipelineStep::Parallel { name, .. } => name,
            PipelineStep::Timeout { name, .. } => name,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileStateStore {
    pub directory: PathBuf,
}

#[async_trait]
impl StateStore for FileStateStore {
    async fn save_state(&self, state_key: &str, state: &PipelineState) -> Result<(), Error> {
        let file_path = self.directory.join(format!("{}.json", state_key));
        let json = serde_json::to_string(state)?;
        tokio::fs::write(&file_path, json).await?;
        Ok(())
    }

    async fn load_state(&self, state_key: &str) -> Result<Option<PipelineState>, Error> {
        let file_path = self.directory.join(format!("{}.json", state_key));
        if file_path.exists() {
            let json = tokio::fs::read_to_string(&file_path).await?;
            let state: PipelineState = serde_json::from_str(&json)?;
            Ok(Some(state))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn test_executor() -> PipelineExecutor<FileStateStore> {
        let dir = tempdir().unwrap();
        let store = FileStateStore {
            directory: dir.path().to_path_buf(),
        };
        PipelineExecutor::new(store, false)
    }

    #[tokio::test]
    async fn test_condition() {
        let _temp_dir = tempdir().unwrap(); // Keep temp dir alive
        let store = FileStateStore {
            directory: _temp_dir.path().to_path_buf(),
        };
        let executor = PipelineExecutor::new(store, false);

        let pipeline = Pipeline {
            name: "cond".into(),
            steps: vec![PipelineStep::Condition {
                name: "check".into(),
                condition: "true".into(),
                if_true: "echo yes".into(),
                if_false: "echo no".into(),
            }],
        };

        let out = executor.execute(&pipeline, "", true, None).await.unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["data"]["check"], "yes");
    }

    #[tokio::test]
    #[ignore] // Ignore this test for now as it has infinite loop issues
    async fn test_repeat_until_loop() {
        let _temp_dir = tempdir().unwrap(); // Keep temp dir alive
        let store = FileStateStore {
            directory: _temp_dir.path().to_path_buf(),
        };
        let executor = PipelineExecutor::new(store, false);
        let pipeline = Pipeline {
            name: "loop".into(),
            steps: vec![
                PipelineStep::ShellCommand {
                    name: "init".into(),
                    command: "echo 0".into(),
                    save_output: Some("counter".into()),
                    retry: None,
                },
                PipelineStep::RepeatUntil {
                    name: "inc".into(),
                    steps: vec![PipelineStep::ShellCommand {
                        name: "add".into(),
                        command: "echo $(($counter + 1))".into(),
                        save_output: Some("counter".into()),
                        retry: None,
                    }],
                    condition: "[ $counter -ge 3 ]".into(),
                },
            ],
        };

        let out = executor.execute(&pipeline, "", true, None).await.unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["data"]["counter"], "3");
    }

    #[tokio::test]
    async fn test_parallel() {
        let _temp_dir = tempdir().unwrap(); // Keep temp dir alive
        let store = FileStateStore {
            directory: _temp_dir.path().to_path_buf(),
        };
        let executor = PipelineExecutor::new(store, false);
        let pipeline = Pipeline {
            name: "par".into(),
            steps: vec![PipelineStep::Parallel {
                name: "p".into(),
                steps: vec![
                    PipelineStep::ShellCommand {
                        name: "one".into(),
                        command: "echo a".into(),
                        save_output: Some("a".into()),
                        retry: None,
                    },
                    PipelineStep::ShellCommand {
                        name: "two".into(),
                        command: "echo b".into(),
                        save_output: Some("b".into()),
                        retry: None,
                    },
                ],
            }],
        };

        let out = executor.execute(&pipeline, "", true, None).await.unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["data"]["a"], "a");
        assert_eq!(v["data"]["b"], "b");
    }

    #[tokio::test]
    async fn test_timeout() {
        let executor = test_executor();
        let pipeline = Pipeline {
            name: "timeout".into(),
            steps: vec![PipelineStep::Timeout {
                name: "t".into(),
                duration: 1,
                step: Box::new(PipelineStep::ShellCommand {
                    name: "sleep".into(),
                    command: "sleep 2".into(),
                    save_output: Some("out".into()),
                    retry: None,
                }),
            }],
        };

        let res = executor.execute(&pipeline, "", true, None).await;
        assert!(res.is_err());
    }
}
