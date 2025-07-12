//! Step execution module
//! 
//! This module handles the execution of individual pipeline steps,
//! delegating to specialized executors for different step types.

use crate::pipeline_executor::{PipelineStep, PipelineState, PipelineFuture};
use crate::pipeline::{
    CommandExecutor, ParallelExecutor, ConditionExecutor,
    LoopExecutor
};
use anyhow::Error;
use std::collections::HashMap;
use anyhow::anyhow;
use log::debug;

/// Handles execution of individual pipeline steps
pub struct StepExecutor;

impl StepExecutor {
    /// Execute a single pipeline step
    pub fn execute_single_step<'a>(
        step: &'a PipelineStep,
        state: &'a mut PipelineState,
    ) -> PipelineFuture<'a> {
        Box::pin(async move {
            debug!("Executing step: {}", step.name());
            
            match step {
                PipelineStep::Command { name: _, command, save_output, retry: _ } => {
                    CommandExecutor::execute_command(command, save_output).await
                }
                PipelineStep::ShellCommand { name: _, command, save_output, retry: _ } => {
                    CommandExecutor::execute_shell_command(command, save_output).await
                }
                PipelineStep::Condition { name, condition, if_true, if_false } => {
                    ConditionExecutor::execute_condition(name, condition, if_true, if_false).await
                }
                PipelineStep::PrintOutput { name: _, value } => {
                    Self::execute_print_output(value).await
                }
                PipelineStep::RepeatUntil { name: _, steps, condition } => {
                    LoopExecutor::execute_repeat_until(steps, condition, state).await
                }
                PipelineStep::ForEach { name, items, steps } => {
                    LoopExecutor::execute_for_each(name, items, steps, state).await
                }
                PipelineStep::TryCatch { name: _, try_steps, catch_steps, finally_steps } => {
                    Self::execute_try_catch(try_steps, catch_steps, finally_steps, state).await
                }
                PipelineStep::Timeout { name: _, duration, step } => {
                    Self::execute_timeout(*duration, step, state).await
                }
                PipelineStep::Parallel { name: _, steps } => {
                    ParallelExecutor::execute_parallel_steps(steps, state).await
                }
                _ => Err(anyhow!("Unknown step type")),
            }
        })
    }

    /// Execute print output step
    async fn execute_print_output(value: &str) -> Result<HashMap<String, String>, Error> {
        println!("{}", value);
        Ok(HashMap::new())
    }

    /// Execute try-catch step
    pub async fn execute_try_catch(
        try_steps: &[PipelineStep],
        catch_steps: &[PipelineStep],
        finally_steps: &[PipelineStep],
        state: &mut PipelineState,
    ) -> Result<HashMap<String, String>, Error> {
        let mut result = HashMap::new();
        
        let try_result = async {
            for sub_step in try_steps {
                let step_result = Self::execute_single_step(sub_step, state).await?;
                state.data.extend(step_result);
            }
            Ok(()) as Result<(), Error>
        }.await;

        match try_result {
            Ok(_) => {
                result.insert("try_result".to_string(), "success".to_string());
            }
            Err(e) => {
                result.insert("try_result".to_string(), "failure".to_string());
                result.insert("error".to_string(), e.to_string());
                for sub_step in catch_steps {
                    let step_result = Self::execute_single_step(sub_step, state).await?;
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

    /// Execute timeout step
    pub async fn execute_timeout(
        duration: u64,
        step: &PipelineStep,
        state: &mut PipelineState,
    ) -> Result<HashMap<String, String>, Error> {
        use tokio::time::{timeout, Duration};
        
        let duration = Duration::from_secs(duration);
        let timeout_result = timeout(duration, Self::execute_single_step(step, state)).await;

        match timeout_result {
            Ok(step_result) => step_result,
            Err(_) => Err(anyhow!(
                "Step timed out after {} seconds",
                duration.as_secs()
            )),
        }
    }
}
