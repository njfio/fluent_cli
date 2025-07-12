//! Parallel execution module
//! 
//! This module handles the execution of parallel pipeline steps,
//! managing concurrent execution and result aggregation.

use crate::pipeline_executor::{PipelineStep, PipelineState};
use crate::pipeline::step_executor::StepExecutor;
use anyhow::Error;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::task::JoinSet;
use log::debug;

/// Handles execution of parallel pipeline steps
pub struct ParallelExecutor;

impl ParallelExecutor {
    /// Execute multiple steps in parallel
    pub async fn execute_parallel_steps(
        steps: &[PipelineStep],
        state: &mut PipelineState,
    ) -> Result<HashMap<String, String>, Error> {
        debug!("Executing parallel steps: {} steps", steps.len());
        
        let state_arc = Arc::new(tokio::sync::Mutex::new(state.clone()));
        let mut set = JoinSet::new();

        for sub_step in steps.iter().cloned() {
            let state_clone = Arc::clone(&state_arc);
            set.spawn(async move {
                let mut guard = state_clone.lock().await;
                StepExecutor::execute_single_step(&sub_step, &mut guard).await
            });
        }

        let mut combined_results = HashMap::new();
        while let Some(result) = set.join_next().await {
            match result {
                Ok(Ok(step_result)) => {
                    combined_results.extend(step_result);
                }
                Ok(Err(e)) => {
                    combined_results.insert(
                        format!("error_{}", combined_results.len()),
                        e.to_string(),
                    );
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
        state.data.extend(combined_results.clone());

        Ok(combined_results)
    }

}
