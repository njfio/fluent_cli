//! Loop execution module
//! 
//! This module handles the execution of loop-based pipeline steps,
//! including repeat-until loops and for-each iterations.

use crate::pipeline_executor::{PipelineStep, PipelineState};
use crate::pipeline::step_executor::StepExecutor;
use crate::pipeline::condition_executor::ConditionExecutor;
use anyhow::Error;
use std::collections::HashMap;
use log::debug;

/// Handles execution of loop-based pipeline steps
pub struct LoopExecutor;

impl LoopExecutor {
    /// Execute a repeat-until loop
    pub async fn execute_repeat_until(
        steps: &[PipelineStep],
        condition: &str,
        state: &mut PipelineState,
    ) -> Result<HashMap<String, String>, Error> {
        debug!("Executing repeat-until loop with condition: {}", condition);
        
        let result = HashMap::new();
        loop {
            // Execute all steps in the loop
            for sub_step in steps {
                let step_result = StepExecutor::execute_single_step(sub_step, state).await?;
                state.data.extend(step_result);
            }

            // Check the exit condition
            let condition_result = Self::evaluate_loop_condition(condition, &state.data).await?;
            if condition_result {
                debug!("Loop condition met, exiting loop");
                break;
            }
        }
        Ok(result)
    }

    /// Execute a for-each loop
    pub async fn execute_for_each(
        name: &str,
        items: &str,
        steps: &[PipelineStep],
        state: &mut PipelineState,
    ) -> Result<HashMap<String, String>, Error> {
        debug!("Executing for-each loop: {} with items: {}", name, items);
        
        let mut result = Vec::new();
        for item in items.split(',') {
            let item = item.trim();
            debug!("Processing item: {}", item);
            
            // Set the current item in state
            state.data.insert("ITEM".to_string(), item.to_string());
            
            // Execute all steps for this item
            for sub_step in steps {
                let step_result = StepExecutor::execute_single_step(sub_step, state).await?;
                state.data.extend(step_result);
            }
            
            // Collect the result for this item
            result.push(state.data.get("ITEM").unwrap_or(&item.to_string()).clone());
        }
        
        // Clean up the ITEM variable
        state.data.remove("ITEM");
        
        Ok(HashMap::from([(name.to_string(), result.join(", "))]))
    }

    /// Execute a for-each loop with custom item variable name
    pub async fn execute_for_each_with_variable(
        name: &str,
        items: &str,
        item_variable: &str,
        steps: &[PipelineStep],
        state: &mut PipelineState,
    ) -> Result<HashMap<String, String>, Error> {
        debug!("Executing for-each loop: {} with items: {} using variable: {}", 
               name, items, item_variable);
        
        let mut result = Vec::new();
        for item in items.split(',') {
            let item = item.trim();
            debug!("Processing item: {} -> {}", item, item_variable);
            
            // Set the current item in state with custom variable name
            state.data.insert(item_variable.to_string(), item.to_string());
            
            // Execute all steps for this item
            for sub_step in steps {
                let step_result = StepExecutor::execute_single_step(sub_step, state).await?;
                state.data.extend(step_result);
            }
            
            // Collect the result for this item
            result.push(state.data.get(item_variable).unwrap_or(&item.to_string()).clone());
        }
        
        // Clean up the item variable
        state.data.remove(item_variable);
        
        Ok(HashMap::from([(name.to_string(), result.join(", "))]))
    }

    /// Execute a while loop
    pub async fn execute_while_loop(
        condition: &str,
        steps: &[PipelineStep],
        state: &mut PipelineState,
        max_iterations: Option<usize>,
    ) -> Result<HashMap<String, String>, Error> {
        debug!("Executing while loop with condition: {}", condition);
        
        let mut iterations = 0;
        let max_iter = max_iterations.unwrap_or(1000); // Safety limit
        
        while iterations < max_iter {
            // Check the condition first
            let condition_result = Self::evaluate_loop_condition(condition, &state.data).await?;
            if !condition_result {
                debug!("While loop condition not met, exiting loop");
                break;
            }

            // Execute all steps in the loop
            for sub_step in steps {
                let step_result = StepExecutor::execute_single_step(sub_step, state).await?;
                state.data.extend(step_result);
            }
            
            iterations += 1;
        }
        
        if iterations >= max_iter {
            debug!("While loop reached maximum iterations: {}", max_iter);
        }
        
        Ok(HashMap::from([("iterations".to_string(), iterations.to_string())]))
    }

    /// Evaluate a loop condition with variable substitution
    async fn evaluate_loop_condition(
        condition: &str,
        state_data: &HashMap<String, String>,
    ) -> Result<bool, Error> {
        // Expand variables in the condition
        let mut expanded_condition = condition.to_string();
        for (key, value) in state_data {
            expanded_condition = expanded_condition.replace(&format!("${{{}}}", key), value);
            expanded_condition = expanded_condition.replace(&format!("${}", key), value);
        }
        
        debug!("Evaluating loop condition: {} -> {}", condition, expanded_condition);
        
        // Use the condition executor to evaluate
        ConditionExecutor::evaluate_condition(&expanded_condition).await
    }

    /// Execute a counted loop (for i in range)
    pub async fn execute_counted_loop(
        name: &str,
        start: i32,
        end: i32,
        step: i32,
        counter_variable: &str,
        steps: &[PipelineStep],
        state: &mut PipelineState,
    ) -> Result<HashMap<String, String>, Error> {
        debug!("Executing counted loop: {} from {} to {} step {}", 
               name, start, end, step);
        
        let mut current = start;
        let mut iterations = 0;
        
        while (step > 0 && current <= end) || (step < 0 && current >= end) {
            // Set the counter variable
            state.data.insert(counter_variable.to_string(), current.to_string());
            
            // Execute all steps for this iteration
            for sub_step in steps {
                let step_result = StepExecutor::execute_single_step(sub_step, state).await?;
                state.data.extend(step_result);
            }
            
            current += step;
            iterations += 1;
            
            // Safety check to prevent infinite loops
            if iterations > 10000 {
                debug!("Counted loop reached safety limit of 10000 iterations");
                break;
            }
        }
        
        // Clean up the counter variable
        state.data.remove(counter_variable);
        
        Ok(HashMap::from([
            (name.to_string(), format!("Completed {} iterations", iterations)),
            ("iterations".to_string(), iterations.to_string()),
        ]))
    }
}
