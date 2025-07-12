//! Condition execution module
//! 
//! This module handles the execution of conditional pipeline steps,
//! evaluating conditions and executing appropriate branches.

use anyhow::{anyhow, Error};
use std::collections::HashMap;
use tokio::process::Command as TokioCommand;
use log::debug;

/// Handles execution of conditional pipeline steps
pub struct ConditionExecutor;

impl ConditionExecutor {
    /// Execute a conditional step
    pub async fn execute_condition(
        name: &str,
        condition: &str,
        if_true: &str,
        if_false: &str,
    ) -> Result<HashMap<String, String>, Error> {
        debug!("Executing condition: {}", condition);
        
        let condition_result = Self::evaluate_condition(condition).await?;
        let command_to_run = if condition_result { if_true } else { if_false };
        
        debug!("Condition result: {}, executing: {}", condition_result, command_to_run);
        
        let output = TokioCommand::new("sh")
            .arg("-c")
            .arg(command_to_run)
            .output()
            .await?;

        let stdout = String::from_utf8(output.stdout)?;
        Ok(HashMap::from([(name.to_string(), stdout.trim().to_string())]))
    }

    /// Evaluate a condition and return boolean result
    pub async fn evaluate_condition(condition: &str) -> Result<bool, Error> {
        debug!("Evaluating condition: {}", condition);
        
        let condition_result = TokioCommand::new("sh")
            .arg("-c")
            .arg(condition)
            .status()
            .await?
            .success();
            
        debug!("Condition '{}' evaluated to: {}", condition, condition_result);
        Ok(condition_result)
    }

    /// Evaluate a condition with variable expansion
    pub async fn evaluate_condition_with_expansion(
        condition: &str,
        state_data: &HashMap<String, String>,
    ) -> Result<bool, Error> {
        let expanded_condition = Self::expand_variables(condition, state_data)?;
        debug!("Evaluating expanded condition: {}", expanded_condition);

        // Use absolute path to bash and clear environment for security
        let bash_path = which::which("bash")
            .map_err(|_| anyhow!("bash command not found in PATH"))?;

        let output = TokioCommand::new(bash_path)
            .arg("-c")
            .arg(format!(
                "if {}; then exit 0; else exit 1; fi",
                expanded_condition
            ))
            .env_clear() // Clear environment for security
            .env("PATH", "/usr/bin:/bin:/usr/local/bin") // Minimal but functional PATH
            .output()
            .await
            .map_err(|e| anyhow!("Failed to execute condition evaluation: {}", e))?;

        Ok(output.status.success())
    }

    /// Execute conditional step with variable expansion
    pub async fn execute_condition_with_expansion(
        name: &str,
        condition: &str,
        if_true: &str,
        if_false: &str,
        state_data: &HashMap<String, String>,
    ) -> Result<HashMap<String, String>, Error> {
        debug!("Executing condition with expansion: {}", condition);
        
        let condition_result = Self::evaluate_condition_with_expansion(condition, state_data).await?;
        let command_to_run = if condition_result { if_true } else { if_false };
        let expanded_command = Self::expand_variables(command_to_run, state_data)?;
        
        debug!("Condition result: {}, executing: {}", condition_result, expanded_command);
        
        let output = TokioCommand::new("sh")
            .arg("-c")
            .arg(&expanded_command)
            .output()
            .await?;

        let stdout = String::from_utf8(output.stdout)?;
        Ok(HashMap::from([(name.to_string(), stdout.trim().to_string())]))
    }

    /// Simple variable expansion helper
    fn expand_variables(
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
