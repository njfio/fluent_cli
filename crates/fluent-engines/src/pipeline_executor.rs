use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;

use std::io::Write;
use tokio::process::Command as TokioCommand;

use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tempfile;

use anyhow::{anyhow, Error};
use async_trait::async_trait;
use log::{debug, error, info, warn};
use std::sync::Arc;
use tokio::process::Command;

use tokio::task::JoinSet;
use tokio::time::timeout;
use uuid::Uuid;
use schemars::JsonSchema;
use jsonschema::{Draft, JSONSchema};
use serde_yaml;
use serde_json;

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
    max_attempts: u32,
    delay_ms: u64,
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

pub fn validate_pipeline_yaml(yaml: &str) -> Result<(), Error> {
    let value: serde_yaml::Value = serde_yaml::from_str(yaml)?;
    let schema = pipeline_schema();
    let compiled = JSONSchema::options()
        .with_draft(Draft::Draft7)
        .compile(&serde_json::to_value(&schema)?)?;
    let instance = serde_json::to_value(&value)?;
    compiled
        .validate(&instance)
        .map(|_| ())
        .map_err(|errors| anyhow!(errors.map(|e| e.to_string()).collect::<Vec<_>>().join("; ")))
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

type PipelineFuture<'a> =
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
                    let expanded_command = self.expand_variables(command, &state.data).await?;
                    self.execute_command(&expanded_command, save_output, retry)
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
                    let expanded_command = self.expand_variables(command, &state.data).await?;
                    self.execute_shell_command(&expanded_command, save_output, retry)
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
                    let expanded_condition = self.expand_variables(condition, &state.data).await?;
                    if self.evaluate_condition(&expanded_condition).await? {
                        debug!("Condition is true, executing: {}", if_true);
                        let expanded_command = self.expand_variables(if_true, &state.data).await?;
                        self.execute_shell_command(&expanded_command, &None, &None)
                            .await
                    } else {
                        debug!("Condition is false, executing: {}", if_false);
                        let expanded_command = self.expand_variables(if_false, &state.data).await?;
                        self.execute_shell_command(&expanded_command, &None, &None)
                            .await
                    }
                }

                PipelineStep::PrintOutput { name, value } => {
                    debug!("Executing PrintOutput step: {}", name);
                    let expanded_value = self.expand_variables(value, &state.data).await?;
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
                    let input_data = self.expand_variables(input, &state.data).await?;
                    debug!("Expanded input data: {}", input_data);
                    let mut results = Vec::new();

                    for item in input_data.split(',') {
                        let item = item.trim();
                        debug!("Processing item: {}", item);
                        let expanded_command = self.expand_variables(command, &state.data).await?;
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
                    let expanded_prompt = self.expand_variables(prompt, &state.data).await?;
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
                    loop {
                        for sub_step in steps {
                            let step_result = self.execute_step(sub_step, state).await?;
                            state.data.extend(step_result);
                        }

                        let expanded_condition =
                            self.expand_variables(condition, &state.data).await?;
                        if self.evaluate_condition(&expanded_condition).await? {
                            break;
                        }
                    }
                    Ok(HashMap::new())
                }

                PipelineStep::ForEach { name, items, steps } => {
                    debug!("Executing ForEach step: {}", name);
                    debug!("Items: {}", items);
                    debug!("Steps: {:?}", steps);
                    let items_list = self.expand_variables(items, &state.data).await?;
                    let mut results = Vec::new();

                    for item in items_list.split(',') {
                        let item = item.trim();
                        state.data.insert("ITEM".to_string(), item.to_string());

                        for sub_step in steps {
                            let step_result = self.execute_step(sub_step, state).await?;
                            state.data.extend(step_result);
                        }

                        results.push(state.data.get("ITEM").unwrap_or(&item.to_string()).clone());
                    }

                    state.data.remove("ITEM");
                    Ok(HashMap::from([(name.clone(), results.join(", "))]))
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
                    let mut result = HashMap::new();
                    let try_result = async {
                        for sub_step in try_steps {
                            let step_result = self.execute_step(sub_step, state).await?;
                            state.data.extend(step_result);
                        }
                        Ok(()) as Result<(), Error>
                    }
                    .await;

                    match try_result {
                        Ok(_) => {
                            result.insert("try_result".to_string(), "success".to_string());
                        }
                        Err(e) => {
                            result.insert("try_result".to_string(), "failure".to_string());
                            result.insert("error".to_string(), e.to_string());
                            for sub_step in catch_steps {
                                let step_result = self.execute_step(sub_step, state).await?;
                                state.data.extend(step_result);
                            }
                        }
                    }

                    for sub_step in finally_steps {
                        let step_result = self.execute_step(sub_step, state).await?;
                        state.data.extend(step_result);
                    }

                    Ok(result)
                }

                PipelineStep::Parallel { name, steps } => {
                    self.execute_parallel_steps(name, steps, state).await
                }

                PipelineStep::Timeout {
                    name,
                    duration,
                    step,
                } => {
                    debug!("Executing Timeout step: {}", name);
                    debug!("Duration: {}", duration);
                    debug!("Step: {:?}", step);
                    let duration = Duration::from_secs(*duration);

                    let timeout_result = timeout(duration, self.execute_step(step, state)).await;

                    match timeout_result {
                        Ok(step_result) => {
                            let result = step_result?;
                            Ok(result)
                        }
                        Err(_) => Err(anyhow!(
                            "Step timed out after {} seconds",
                            duration.as_secs()
                        )),
                    }
                }

                _ => Ok(HashMap::new()),
            }
        })
    }

    async fn execute_parallel_steps(
        &self,
        name: &str,
        steps: &[PipelineStep],
        state: &mut PipelineState,
    ) -> Result<HashMap<String, String>, Error> {
        debug!("Executing Parallel step: {}", name);
        debug!("Steps: {:?}", steps);

        // Pre-expand variables for all steps
        let expanded_steps: Vec<PipelineStep> = futures::future::try_join_all(
            steps
                .iter()
                .map(|step| self.expand_variables_in_step(step, &state.data)),
        )
        .await?;

        let state_arc = Arc::new(tokio::sync::Mutex::new(state.clone()));
        let mut set = JoinSet::new();

        for sub_step in expanded_steps {
            let state_clone = Arc::clone(&state_arc);

            set.spawn(async move {
                let mut state_guard = state_clone.lock().await;
                Self::execute_single_step(&sub_step, &mut state_guard).await
            });
        }

        let mut combined_results = HashMap::new();
        while let Some(result) = set.join_next().await {
            match result {
                Ok(Ok(step_result)) => {
                    combined_results.extend(step_result);
                }
                Ok(Err(e)) => {
                    combined_results
                        .insert(format!("error_{}", combined_results.len()), e.to_string());
                }
                Err(e) => {
                    combined_results.insert(
                        format!("join_error_{}", combined_results.len()),
                        e.to_string(),
                    );
                }
            }
        }

        // Merge the results back into the main state
        let state_guard = state_arc.lock().await;
        state.data.extend(state_guard.data.clone());
        state.data.extend(combined_results);

        Ok(HashMap::from([(
            name.to_string(),
            "Parallel execution completed".to_string(),
        )]))
    }

    async fn expand_variables_in_step(
        &self,
        step: &PipelineStep,
        state_data: &HashMap<String, String>,
    ) -> Result<PipelineStep, Error> {
        match step {
            PipelineStep::Command {
                name,
                command,
                save_output,
                retry,
            } => {
                let expanded_command = self.expand_variables(command, state_data).await?;
                Ok(PipelineStep::Command {
                    name: name.clone(),
                    command: expanded_command,
                    save_output: save_output.clone(),
                    retry: retry.clone(),
                })
            }
            PipelineStep::ShellCommand {
                name,
                command,
                save_output,
                retry,
            } => {
                let expanded_command = self.expand_variables(command, state_data).await?;
                Ok(PipelineStep::ShellCommand {
                    name: name.clone(),
                    command: expanded_command,
                    save_output: save_output.clone(),
                    retry: retry.clone(),
                })
            }
            // For other step types, we can simply clone them as they are
            _ => Ok(step.clone()),
        }
    }

    fn execute_single_step<'a>(
        step: &'a PipelineStep,
        state: &'a mut PipelineState,
    ) -> PipelineFuture<'a> {
        Box::pin(async move {
            match step {
                PipelineStep::Command {
                    name: _,
                    command,
                    save_output,
                    retry: _,
                } => {
                    let output = Command::new("sh").arg("-c").arg(command).output().await?;

                    let stdout = String::from_utf8(output.stdout)?;
                    let mut result = HashMap::new();
                    if let Some(key) = save_output {
                        result.insert(key.clone(), stdout.trim().to_string());
                    }
                    Ok(result)
                }
                PipelineStep::ShellCommand {
                    name: _,
                    command,
                    save_output,
                    retry: _,
                } => {
                    let output = Command::new("sh").arg("-c").arg(command).output().await?;

                    let stdout = String::from_utf8(output.stdout)?;
                    let mut result = HashMap::new();
                    if let Some(key) = save_output {
                        result.insert(key.clone(), stdout.trim().to_string());
                    }
                    Ok(result)
                }
                PipelineStep::Condition {
                    name,
                    condition,
                    if_true,
                    if_false,
                } => {
                    let condition_result = Command::new("sh")
                        .arg("-c")
                        .arg(condition)
                        .status()
                        .await?
                        .success();

                    let command_to_run = if condition_result { if_true } else { if_false };
                    let output = Command::new("sh")
                        .arg("-c")
                        .arg(command_to_run)
                        .output()
                        .await?;

                    let stdout = String::from_utf8(output.stdout)?;
                    Ok(HashMap::from([(name.clone(), stdout.trim().to_string())]))
                }
                PipelineStep::PrintOutput { name: _, value } => {
                    println!("{}", value);
                    Ok(HashMap::new())
                }
                PipelineStep::RepeatUntil {
                    name: _,
                    steps,
                    condition,
                } => {
                    let result = HashMap::new();
                    loop {
                        for sub_step in steps {
                            let step_result = Self::execute_single_step(sub_step, state).await?;
                            state.data.extend(step_result);
                        }

                        let condition_result = Command::new("sh")
                            .arg("-c")
                            .arg(condition)
                            .status()
                            .await?
                            .success();

                        if condition_result {
                            break;
                        }
                    }
                    Ok(result)
                }
                PipelineStep::ForEach { name, items, steps } => {
                    let mut result = Vec::new();
                    for item in items.split(',') {
                        state
                            .data
                            .insert("ITEM".to_string(), item.trim().to_string());
                        for sub_step in steps {
                            let step_result = Self::execute_single_step(sub_step, state).await?;
                            state.data.extend(step_result);
                        }
                        result.push(state.data.get("ITEM").unwrap_or(&item.to_string()).clone());
                    }
                    state.data.remove("ITEM");
                    Ok(HashMap::from([(name.clone(), result.join(", "))]))
                }
                PipelineStep::TryCatch {
                    name: _,
                    try_steps,
                    catch_steps,
                    finally_steps,
                } => {
                    let mut result = HashMap::new();
                    let try_result = async {
                        for sub_step in try_steps {
                            let step_result = Self::execute_single_step(sub_step, state).await?;
                            state.data.extend(step_result);
                        }
                        Ok(()) as Result<(), Error>
                    }
                    .await;

                    match try_result {
                        Ok(_) => {
                            result.insert("try_result".to_string(), "success".to_string());
                        }
                        Err(e) => {
                            result.insert("try_result".to_string(), "failure".to_string());
                            result.insert("error".to_string(), e.to_string());
                            for sub_step in catch_steps {
                                let step_result =
                                    Self::execute_single_step(sub_step, state).await?;
                                state.data.extend(step_result);
                            }
                        }
                    }

                    for sub_step in finally_steps {
                        let step_result = Self::execute_single_step(sub_step, state).await?;
                        state.data.extend(step_result);
                    }

                    Ok(result)
                }
                PipelineStep::Timeout {
                    name: _,
                    duration,
                    step,
                } => {
                    let duration = Duration::from_secs(*duration);
                    let timeout_result =
                        timeout(duration, Self::execute_single_step(step, state)).await;

                    match timeout_result {
                        Ok(step_result) => step_result,
                        Err(_) => Err(anyhow!(
                            "Step timed out after {} seconds",
                            duration.as_secs()
                        )),
                    }
                }
                PipelineStep::Parallel { name: _, steps } => {
                    let state_arc = Arc::new(tokio::sync::Mutex::new(state.clone()));
                    let mut set = JoinSet::new();

                    for sub_step in steps.iter().cloned() {
                        let state_clone = Arc::clone(&state_arc);
                        set.spawn(async move {
                            let mut guard = state_clone.lock().await;
                            Self::execute_single_step(&sub_step, &mut guard).await
                        });
                    }

                    let mut combined_results = HashMap::new();
                    while let Some(result) = set.join_next().await {
                        match result {
                            Ok(Ok(step_result)) => {
                                combined_results.extend(step_result);
                            }
                            Ok(Err(e)) => {
                                combined_results
                                    .insert(format!("error_{}", combined_results.len()), e.to_string());
                            }
                            Err(e) => {
                                combined_results.insert(
                                    format!("join_error_{}", combined_results.len()),
                                    e.to_string(),
                                );
                            }
                        }
                    }

                    let state_guard = state_arc.lock().await;
                    state.data.extend(state_guard.data.clone());
                    state.data.extend(combined_results.clone());

                    Ok(combined_results)
                }
                _ => Err(anyhow!("Unknown step type")),
            }
        })
    }

    async fn execute_command(
        &self,
        command: &str,
        save_output: &Option<String>,
        retry: &Option<RetryConfig>,
    ) -> Result<HashMap<String, String>, Error> {
        debug!("Executing command: {}", command);
        let retry_config = retry.clone().unwrap_or(RetryConfig {
            max_attempts: 1,
            delay_ms: 0,
        });
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
                    tokio::time::sleep(std::time::Duration::from_millis(retry_config.delay_ms))
                        .await;
                }
                Err(e) => {
                    error!(
                        "Command execution failed after {} attempts: {:?}",
                        attempts + 1,
                        e
                    );
                    return Err(e);
                }
            }
        }
    }

    async fn run_command(
        &self,
        command: &str,
        save_output: &Option<String>,
    ) -> Result<HashMap<String, String>, Error> {
        debug!("Running command: {}", command);
        let output = TokioCommand::new("sh")
            .arg("-c")
            .arg(command)
            .output()
            .await
            .map_err(|e| anyhow!("Failed to execute command: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!(
                "Command failed with exit code {:?}. Stderr: {}",
                output.status.code(),
                stderr
            ));
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
        let output = TokioCommand::new("bash")
            .arg(script_path)
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

    async fn evaluate_condition(&self, condition: &str) -> Result<bool, Error> {
        let expanded_condition = self
            .expand_variables(condition, &Default::default())
            .await?;
        debug!("Evaluating expanded condition: {}", expanded_condition);

        let output = TokioCommand::new("bash")
            .arg("-c")
            .arg(format!(
                "if {}; then exit 0; else exit 1; fi",
                expanded_condition
            ))
            .output()
            .await?;

        Ok(output.status.success())
    }

    async fn expand_variables(
        &self,
        input: &str,
        state_data: &HashMap<String, String>,
    ) -> Result<String, Error> {
        debug!("Expanding variables in input: {}", input);
        let mut result = input.to_string();
        for (key, value) in state_data {
            result = result.replace(&format!("${{{}}}", key), value);
        }
        Ok(result)
    }
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
        let store = FileStateStore { directory: dir.path().to_path_buf() };
        PipelineExecutor::new(store, false)
    }

    #[tokio::test]
    async fn test_condition() {
        let executor = test_executor();
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
        assert_eq!(v["data"]["output"], "yes");
    }

    #[tokio::test]
    async fn test_repeat_until_loop() {
        let executor = test_executor();
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
        let executor = test_executor();
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
