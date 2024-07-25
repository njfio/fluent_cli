use std::cell::RefCell;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Pointer;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::rc::Rc;
use tokio::sync::{Mutex, MutexGuard};
use tokio::process::Command as TokioCommand;
use tokio::sync::Mutex as TokioMutex;
use std::io::Write;

use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tempfile::{NamedTempFile, tempdir};

use std::sync::{Arc};
use anyhow::{anyhow, Error};
use tokio::process::Command;
use log::{info, error, warn, debug};
use async_trait::async_trait;
use tokio::fs;
use tokio::io::stdout;
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Pipeline {
    pub name: String,
    pub steps: Vec<PipelineStep>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum PipelineStep {
    Command { name: String, command: String, save_output: Option<String>, retry: Option<RetryConfig> },
    ShellCommand { name: String, command: String, save_output: Option<String>, retry: Option<RetryConfig> },
    Condition { name: String, condition: String, if_true: String, if_false: String },
    Loop { name: String, steps: Vec<PipelineStep>, condition: String },
    Map { name: String, iterable: String, steps: Vec<PipelineStep> },
    SubPipeline { name: String, pipeline: String, with: HashMap<String, String> },
    HumanInput { name: String, prompt: String, save_output: String },
    PrintOutput { name: String, value: String },

}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RetryConfig {
    max_attempts: u32,
    delay_ms: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PipelineState {
    pub current_step: usize,
    pub data: HashMap<String, String>,
    pub run_id: String,
    pub start_time: u64,
}

#[async_trait]
pub trait StateStore {
    async fn save_state(&self, pipeline_name: &str, state: &PipelineState) -> anyhow::Result<()>;
    async fn load_state(&self, pipeline_name: &str) -> anyhow::Result<Option<PipelineState>>;
}

pub struct PipelineExecutor<S: StateStore> {
    // Change state to Arc<Mutex<...>>
    state: Arc<Mutex<PipelineState>>,
    state_store: S,
}

impl<S: StateStore + Clone + std::marker::Sync + std::marker::Send> PipelineExecutor<S> {
    pub fn new(state_store: S) -> Self {
        Self {
            state: Arc::new(Mutex::new(PipelineState {
                current_step: 0,
                data: HashMap::new(),
                run_id: "".to_string(),
                start_time: 0,
            })),
            state_store,
        }
    }


    pub async fn execute(&self, pipeline: &Pipeline, initial_input: &str, force_fresh: bool, provided_run_id: Option<String>) -> Result<String, Error> {
        let run_id = provided_run_id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let state_key = format!("{}-{}", pipeline.name, run_id);
        debug!("Executing pipeline {} with run_id {}", pipeline.name, run_id);

        let mut state = if force_fresh {
            debug!("Forcing fresh state");
            PipelineState {
                current_step: 0,
                data: HashMap::new(),
                run_id: run_id.clone(),
                // ... initialize other fields
                start_time: 0,
            }
        } else {
            debug!("Checking for saved state");
            self.state_store.load_state(&state_key).await?.unwrap_or_else(|| {
                debug!("No saved state found, starting fresh");
                PipelineState {
                    current_step: 0,
                    data: HashMap::new(),
                    run_id: run_id.clone(),
                    // ... initialize other fields
                    start_time: 0,
                }
            })
        };

        state.data.insert("input".to_string(), initial_input.to_string());

        for (index, step) in pipeline.steps.iter().enumerate().skip(state.current_step) {
            debug!("Processing step {} (index {})", step.name(), index);

            state.data.insert("step".to_string(), step.name().to_string());
            state.current_step = index;

            debug!("Calling execute_step for {}", step.name());
            match self.execute_step(step, &state).await {
                Ok(step_result) => {
                    info!("Step {} completed successfully", step.name());
                    state.data.extend(step_result);
                    self.state_store.save_state(state_key.as_str(), &state).await?;
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
            "run_id": state.run_id,
            "current_step": state.current_step,
            "start_time": state.start_time,
            "end_time": end_time,
            "runtime_seconds": runtime,
            "data": state.data,
        });

        Ok(serde_json::to_string_pretty(&output)?)
    }




    async fn execute_step(&self, step: &PipelineStep, state: &PipelineState) -> Result<HashMap<String, String>, Error> {
        debug!("Starting execution of step: {:?}", step);

        let result = match step {
            PipelineStep::Command { name, command, save_output, retry } => {
                debug!("Executing Command step: {}", name);
                let expanded_command = self.expand_variables(command, &state.data).await?;
                let output = self.execute_command(&expanded_command, save_output, retry).await?;
                Ok(output)
            }
            PipelineStep::ShellCommand { name, command, save_output, retry } => {
                debug!("Executing ShellCommand step: {}", name);
                let expanded_command = self.expand_variables(command, &state.data).await?;
                let output = self.execute_shell_command(&expanded_command, save_output, retry).await?;
                Ok(output)
            }
            PipelineStep::Condition { name, condition, if_true, if_false } => {
                debug!("Evaluating Condition step: {}", name);
                let expanded_condition = self.expand_variables(condition, &state.data).await?;
                if self.evaluate_condition(&expanded_condition).await? {
                    debug!("Condition is true, executing: {}", if_true);
                    let expanded_command = self.expand_variables(if_true, &state.data).await?;
                    let output = self.execute_shell_command(&expanded_command, &None, &None).await?;
                    Ok(output)
                } else {
                    debug!("Condition is false, executing: {}", if_false);
                    let expanded_command = self.expand_variables(if_false, &state.data).await?;
                    let output = self.execute_shell_command(&expanded_command, &None, &None).await?;
                    Ok(output)
                }
            }
            PipelineStep::PrintOutput { name, value } => {
                debug!("Executing PrintOutput step: {}", name);
                let expanded_value = self.expand_variables(value, &state.data).await?;
                println!("{}", expanded_value);
                Ok(HashMap::new())  // Return an empty HashMap for PrintOutput
            }
            _ => {
                debug!("Unhandled step type: {:?}", step);
                Err(anyhow!("Unhandled step type"))
            }
        };

        debug!("Finished execution of step: {:?}, result: {:?}", step, result.is_ok());
        result
    }


    async fn execute_with_retry<F, Fut>(
        &self,
        config: &Option<RetryConfig>,
        f: F,
    ) -> anyhow::Result<()>
        where
            F: Fn() -> Fut,
            Fut: std::future::Future<Output = anyhow::Result<()>> + Send,
    {
        debug!("Executing with retry");
        let config = config.clone().unwrap_or(RetryConfig {
            max_attempts: 1,
            delay_ms: 0,
        });
        let mut attempts = 0;

        loop {
            // Box the future returned by f()
            match Box::pin(f()).await {
                Ok(_) => return Ok(()),
                Err(e) if attempts < config.max_attempts => {
                    attempts += 1;
                    warn!("Attempt {} failed: {:?}. Retrying...", attempts, e);
                    tokio::time::sleep(std::time::Duration::from_millis(config.delay_ms)).await;
                }
                Err(e) => return Err(e),
            }
        }
    }

    async fn execute_command(&self, command: &str, save_output: &Option<String>, retry: &Option<RetryConfig>) -> Result<HashMap<String, String>, Error> {
        debug!("Executing command: {}", command);
        let retry_config = retry.clone().unwrap_or(RetryConfig { max_attempts: 1, delay_ms: 0 });
        let mut attempts = 0;

        loop {
            debug!("Attempt {} to execute command", attempts + 1);
            match self.run_command(command, save_output).await {
                Ok(output) => {
                    debug!("Command executed successfully");
                    return Ok(output);
                }
                Err(e) if attempts < retry_config.max_attempts => {
                    attempts += 1;
                    warn!("Attempt {} failed: {:?}. Retrying...", attempts, e);
                    tokio::time::sleep(std::time::Duration::from_millis(retry_config.delay_ms)).await;
                }
                Err(e) => {
                    error!("Command execution failed after {} attempts: {:?}", attempts + 1, e);
                    return Err(e);
                }
            }
        }
    }

    async fn run_command(&self, command: &str, save_output: &Option<String>) -> Result<HashMap<String, String>, Error> {
        debug!("Running command: {}", command);
        let output = TokioCommand::new("sh")
            .arg("-c")
            .arg(command)
            .output()
            .await
            .map_err(|e| anyhow!("Failed to execute command: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Command failed with exit code {:?}. Stderr: {}", output.status.code(), stderr));
        }

        let stdout = String::from_utf8(output.stdout)
            .map_err(|e| anyhow!("Failed to parse command output: {}", e))?;

        debug!("Command output: {}", stdout);

        let mut result = HashMap::new();
        if let Some(save_key) = save_output {
            result.insert(save_key.clone(), stdout.trim().to_string());
            debug!("Saved output to key: {}", save_key);
        }

        Ok(result)
    }





    async fn execute_shell_command(&self, command: &str, save_output: &Option<String>, retry: &Option<RetryConfig>) -> Result<HashMap<String, String>, Error> {
        debug!("Executing shell command: {}", command);

        // Create a temporary file
        let mut temp_file = NamedTempFile::new()?;
        writeln!(temp_file.as_file_mut(), "{}", command)?;

        let retry_config = retry.clone().unwrap_or(RetryConfig { max_attempts: 1, delay_ms: 0 });
        let mut attempts = 0;

        loop {
            debug!("Attempt {} to execute shell command", attempts + 1);
            match self.run_shell_command(temp_file.path(), save_output).await {
                Ok(output) => {
                    debug!("Shell command executed successfully");
                    return Ok(output);
                }
                Err(e) if attempts < retry_config.max_attempts => {
                    attempts += 1;
                    warn!("Attempt {} failed: {:?}. Retrying...", attempts, e);
                    tokio::time::sleep(std::time::Duration::from_millis(retry_config.delay_ms)).await;
                }
                Err(e) => {
                    error!("Shell command execution failed after {} attempts: {:?}", attempts + 1, e);
                    return Err(e);
                }
            }
        }
    }

    async fn run_shell_command(&self, script_path: &Path, save_output: &Option<String>) -> Result<HashMap<String, String>, Error> {
        debug!("Running shell command from file: {:?}", script_path);
        let output = TokioCommand::new("bash")
            .arg(script_path)
            .output()
            .await
            .map_err(|e| anyhow!("Failed to execute shell command: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Shell command failed with exit code {:?}. Stderr: {}", output.status.code(), stderr));
        }

        let stdout = String::from_utf8(output.stdout)
            .map_err(|e| anyhow!("Failed to parse command output: {}", e))?;

        debug!("Shell command output: {}", stdout);

        let mut result = HashMap::new();
        if let Some(save_key) = save_output {
            result.insert(save_key.clone(), stdout.trim().to_string());
            debug!("Saved output to key: {}", save_key);
        }

        Ok(result)
    }



    async fn evaluate_condition(&self, condition: &str) -> Result<bool, Error> {
        let expanded_condition = self.expand_variables(condition, &Default::default()).await?;
        debug!("Evaluating expanded condition: {}", expanded_condition);

        let output = TokioCommand::new("bash")
            .arg("-c")
            .arg(format!("if {}; then exit 0; else exit 1; fi", expanded_condition))
            .output()
            .await?;

        Ok(output.status.success())
    }

    async fn expand_variables(&self, input: &str, state_data: &HashMap<String, String>) -> Result<String, Error> {
        debug!("Expanding variables in input: {}", input);
        let mut result = input.to_string();
        for (key, value) in state_data {
            result = result.replace(&format!("${{{}}}", key), value);
        }
        Ok(result)
    }

}


fn evaluate_condition(condition: &str, state: &HashMap<String, String>) -> bool {
    // Implement condition evaluation
    // This could be a simple string contains check, or something more complex
    unimplemented!()
}
impl PipelineStep {
    fn name(&self) -> &str {
        match self {
            PipelineStep::Command { name, .. } => name,
            PipelineStep::ShellCommand { name, .. } => name,
            PipelineStep::Condition { name, .. } => name,
            PipelineStep::Loop { name, .. } => name,
            PipelineStep::Map { name, .. } => name,
            PipelineStep::SubPipeline { name, .. } => name,
            PipelineStep::HumanInput { name, .. } => name,
            PipelineStep::PrintOutput { name, .. } => name,
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