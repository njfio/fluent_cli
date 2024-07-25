use std::cell::RefCell;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Pointer;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::rc::Rc;
use tokio::sync::{Mutex, MutexGuard};
use tokio::process::Command as TokioCommand;
use tokio::sync::Mutex as TokioMutex;


use std::sync::{Arc};
use anyhow::{anyhow, Error};
use tokio::process::Command;
use log::{info, error, warn, debug};
use async_trait::async_trait;
use tokio::fs;
use tokio::io::stdout;

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

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PipelineState {
    pub current_step: usize,
    pub data: HashMap<String, String>,
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
            })),
            state_store,
        }
    }

    pub async fn execute(&self, pipeline: &Pipeline, initial_input: &str) -> Result<String, Error> {
        {
            debug!("Executing pipeline {}", pipeline.name);
            let mut state = self.state.lock().await;
            info!("Starting pipeline execution from step {}", state.current_step);
            state.data.insert("input".to_string(), initial_input.to_string());
        }

        debug!("Checking for saved state");
        if let Some(saved_state) = self.state_store.load_state(&pipeline.name).await? {
            debug!("Loaded saved state: {:?}", saved_state);
            *self.state.lock().await = saved_state;
        } else {
            debug!("No saved state found, initializing with default state");
            let mut state = self.state.lock().await;
            state.current_step = 0;
            state.data.insert("input".to_string(), initial_input.to_string());
        }

        let current_step = {
            let state = self.state.lock().await;
            state.current_step
        };

        for (index, step) in pipeline.steps.iter().enumerate().skip(current_step) {
            debug!("Processing step {} (index {})", step.name(), index);

            {
                let mut state = self.state.lock().await;
                debug!("Updating state for step {}", step.name());
                state.data.insert("step".to_string(), step.name().to_string());
                state.current_step = index;
            }


            debug!("Calling execute_step for {}", step.name());
            match self.execute_step(step).await {
                Ok(_) => {
                    info!("Step {} completed successfully", step.name());
                    self.state_store.save_state(&pipeline.name, &*self.state.lock().await).await?;
                }
                Err(e) => {
                    error!("Error executing step {}: {:?}", step.name(), e);
                    return Err(e);
                }
            }
        }

        // Determine what to return as the final output
        let final_state = self.state.lock().await;
        Ok(serde_json::to_string_pretty(&*final_state)?)

    }




    async fn execute_step(&self, step: &PipelineStep) -> Result<(), Error> {
        debug!("Starting execution of step: {:?}", step);

        let result = match step {
            PipelineStep::Command { name, command, save_output, retry } => {
                debug!("Executing Command step: {}", name);
                let expanded_command = self.expand_variables(command).await?;
                self.execute_command(&expanded_command, save_output, retry).await
            }
            PipelineStep::ShellCommand { name, command, save_output, retry } => {
                debug!("Executing ShellCommand step: {}", name);
                let expanded_command = self.expand_variables(command).await?;
                self.execute_shell_command(&expanded_command, save_output, retry).await
            }
            PipelineStep::Condition { name, condition, if_true, if_false } => {
                debug!("Evaluating Condition step: {}", name);
                if self.evaluate_condition(condition).await? {
                    debug!("Condition is true, executing: {}", if_true);
                    let expanded_command = self.expand_variables(if_true).await?;
                    self.execute_shell_command(&expanded_command, &None, &None).await
                } else {
                    debug!("Condition is false, executing: {}", if_false);
                    let expanded_command = self.expand_variables(if_false).await?;
                    self.execute_shell_command(&expanded_command, &None, &None).await
                }
            }
            PipelineStep::PrintOutput { name, value } => {
                debug!("Executing PrintOutput step: {}", name);
                let expanded_value = self.expand_variables(value).await?;
                println!("{}", expanded_value);
                Ok(())
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

    async fn execute_command(&self, command: &str, save_output: &Option<String>, retry: &Option<RetryConfig>) -> Result<(), Error> {
        debug!("Executing command: {}", command);
        let retry_config = retry.clone().unwrap_or(RetryConfig { max_attempts: 1, delay_ms: 0 });
        let mut attempts = 0;

        loop {
            debug!("Attempt {} to execute command", attempts + 1);
            match self.run_command(command, save_output).await {
                Ok(_) => {
                    debug!("Command executed successfully");
                    return Ok(());
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

    async fn run_command(&self, command: &str, save_output: &Option<String>) -> Result<(), Error> {
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

        if let Some(save_key) = save_output {
            let mut state = self.state.lock().await;
            state.data.insert(save_key.clone(), stdout);
            debug!("Saved output to key: {}", save_key);
        }

        Ok(())
    }


    async fn execute_shell_command(&self, command: &str, save_output: &Option<String>, retry: &Option<RetryConfig>) -> Result<(), Error> {
        // Similar to execute_command, but with any shell-specific logic
        debug!("Executing shell command: {}", command);
        self.execute_command(command, save_output, retry).await
    }


    async fn evaluate_condition(&self, condition: &str) -> Result<bool, Error> {
        let expanded_condition = self.expand_variables(condition).await?;
        debug!("Evaluating expanded condition: {}", expanded_condition);

        let output = TokioCommand::new("bash")
            .arg("-c")
            .arg(format!("if {}; then exit 0; else exit 1; fi", expanded_condition))
            .output()
            .await?;

        Ok(output.status.success())
    }

    async fn expand_variables(&self, input: &str) -> Result<String, Error> {
        debug!("Expanding variables in input: {}", input);
        let state = self.state.lock().await;
        let mut result = input.to_string();
        for (key, value) in &state.data {
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
    async fn save_state(&self, pipeline_name: &str, state: &PipelineState) -> anyhow::Result<()> {
        let file_path = self.directory.join(format!("{}.json", pipeline_name));
        debug!("Attempting to save state to file: {:?}", file_path);

        // Ensure the directory exists
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let json = serde_json::to_string(state)?;
        fs::write(&file_path, json).await?;
        debug!("State saved successfully to {:?}", file_path);
        Ok(())
    }

    async fn load_state(&self, pipeline_name: &str) -> anyhow::Result<Option<PipelineState>> {
        let file_path = self.directory.join(format!("{}.json", pipeline_name));
        debug!("Attempting to load state from file: {:?}", file_path);

        if file_path.exists() {
            let json = fs::read_to_string(&file_path).await?;
            let state: PipelineState = serde_json::from_str(&json)?;
            debug!("State loaded successfully from {:?}", file_path);
            Ok(Some(state))
        } else {
            debug!("No existing state file found at {:?}", file_path);
            Ok(None)
        }
    }
}