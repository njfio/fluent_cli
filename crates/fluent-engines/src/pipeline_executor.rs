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
    SubPipeline { name: String, pipeline: String, with: HashMap<String, String> },
    Map { name: String, input: String, command: String, save_output: String },
    HumanInTheLoop { name: String, prompt: String, save_output: String },
    RepeatUntil { name: String, steps: Vec<PipelineStep>, condition: String },
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
                start_time: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
            }
        } else {
            match self.state_store.load_state(&state_key).await? {
                Some(saved_state) => {
                    debug!("Resuming from saved state at step {}", saved_state.current_step);
                    saved_state
                },
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
            state.data.insert("input".to_string(), initial_input.to_string());
        }
        state.data.insert("run_id".to_string(), run_id.clone());

        for (index, step) in pipeline.steps.iter().enumerate().skip(state.current_step) {
            debug!("Processing step {} (index {})", step.name(), index);

            state.data.insert("step".to_string(), step.name().to_string());
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




    fn execute_step<'a>(&'a self, step: &'a PipelineStep, state: &'a mut PipelineState) -> Pin<Box<dyn Future<Output = Result<HashMap<String, String>, Error>> + Send + 'a>> {
        Box::pin(async move {
            match step {
                PipelineStep::Command { name, command, save_output, retry } => {
                    debug!("Executing Command step: {}", name);
                    let expanded_command = self.expand_variables(command, &state.data).await?;
                    self.execute_command(&expanded_command, save_output, retry).await
                }
                PipelineStep::ShellCommand { name, command, save_output, retry } => {
                    debug!("Executing ShellCommand step: {}", name);
                    let expanded_command = self.expand_variables(command, &state.data).await?;
                    self.execute_shell_command(&expanded_command, save_output, retry).await
                }
                PipelineStep::Condition { name, condition, if_true, if_false } => {
                    debug!("Evaluating Condition step: {}", name);
                    let expanded_condition = self.expand_variables(condition, &state.data).await?;
                    if self.evaluate_condition(&expanded_condition).await? {
                        debug!("Condition is true, executing: {}", if_true);
                        let expanded_command = self.expand_variables(if_true, &state.data).await?;
                        self.execute_shell_command(&expanded_command, &None, &None).await
                    } else {
                        debug!("Condition is false, executing: {}", if_false);
                        let expanded_command = self.expand_variables(if_false, &state.data).await?;
                        self.execute_shell_command(&expanded_command, &None, &None).await
                    }
                }
                PipelineStep::PrintOutput { name, value } => {
                    debug!("Executing PrintOutput step: {}", name);
                    let expanded_value = self.expand_variables(value, &state.data).await?;
                    println!("{}", expanded_value);
                    Ok(HashMap::new())
                }
                PipelineStep::Map { name, input, command, save_output } => {
                    debug!("Executing Map step: {}", name);
                    let input_data = self.expand_variables(input, &state.data).await?;
                    debug!("Expanded input data: {}", input_data);
                    let mut results = Vec::new();

                    for item in input_data.split(',') {
                        let item = item.trim();
                        debug!("Processing item: {}", item);
                        let expanded_command = self.expand_variables(command, &state.data).await?;
                        let item_command = expanded_command.replace("${ITEM}", item);
                        debug!("Executing command: {}", item_command);
                        match self.execute_shell_command(&item_command, &None, &None).await {
                            Ok(output) => {
                                let new_string = String::new();
                                let result = output.values().next().unwrap_or(&new_string);
                                debug!("Command output: {}", result);
                                results.push(result.to_string());
                            },
                            Err(e) => {
                                error!("Error executing command for item {}: {:?}", item, e);
                            }
                        }
                    }

                    let output = results.join(", ");
                    debug!("Map step result: {}", output);
                    Ok([(save_output.clone(), output)].into_iter().collect())
                }
                PipelineStep::HumanInTheLoop { name, prompt, save_output } => {
                    debug!("Executing HumanInTheLoop step: {}", name);
                    let expanded_prompt = self.expand_variables(prompt, &state.data).await?;
                    println!("{}", expanded_prompt);

                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input)?;

                    Ok([(save_output.clone(), input.trim().to_string())].into_iter().collect())
                }
                PipelineStep::RepeatUntil { name, steps, condition } => {
                    debug!("Executing RepeatUntil step: {}", name);
                    loop {
                        for sub_step in steps {
                            let step_result = self.execute_step(sub_step, state).await?;
                            state.data.extend(step_result);
                        }

                        let expanded_condition = self.expand_variables(condition, &state.data).await?;
                        if self.evaluate_condition(&expanded_condition).await? {
                            break;
                        }
                    }
                    Ok(HashMap::new())
                }
                _ => {
                    Ok(HashMap::new())
                }
            }
        })
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
        let mut temp_file = tempfile::NamedTempFile::new()?;
        writeln!(temp_file.as_file_mut(), "{}", command)?;

        let retry_config = retry.clone().unwrap_or(RetryConfig { max_attempts: 1, delay_ms: 0 });
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
                    tokio::time::sleep(std::time::Duration::from_millis(retry_config.delay_ms)).await;
                }
                Err(e) => {
                    error!("Shell command execution failed after {} attempts: {:?}", attempts + 1, e);
                    return Err(e);
                }
            }
        }
    }


    async fn run_shell_command(&self, script_path: &Path) -> Result<String, Error> {
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

        Ok(stdout.trim().to_string())
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
            PipelineStep::HumanInTheLoop { name, .. } => name,
            PipelineStep::RepeatUntil { name, .. } => name,
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