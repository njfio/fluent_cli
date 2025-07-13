//! Variable expansion module
//! 
//! This module handles variable expansion in pipeline steps,
//! supporting various variable formats and expansion strategies.

use crate::pipeline_executor::PipelineStep;
use anyhow::Error;
use std::collections::HashMap;
use log::debug;

/// Handles variable expansion in pipeline steps
pub struct VariableExpander;

impl VariableExpander {
    /// Expand variables in a pipeline step
    pub async fn expand_variables_in_step(
        step: &PipelineStep,
        state_data: &HashMap<String, String>,
    ) -> Result<PipelineStep, Error> {
        match step {
            PipelineStep::Command { name, command, save_output, retry } => {
                let expanded_command = Self::expand_variables(command, state_data).await?;
                Ok(PipelineStep::Command {
                    name: name.clone(),
                    command: expanded_command,
                    save_output: save_output.clone(),
                    retry: retry.clone(),
                })
            }
            PipelineStep::ShellCommand { name, command, save_output, retry } => {
                let expanded_command = Self::expand_variables(command, state_data).await?;
                Ok(PipelineStep::ShellCommand {
                    name: name.clone(),
                    command: expanded_command,
                    save_output: save_output.clone(),
                    retry: retry.clone(),
                })
            }
            PipelineStep::Condition { name, condition, if_true, if_false } => {
                let expanded_condition = Self::expand_variables(condition, state_data).await?;
                let expanded_if_true = Self::expand_variables(if_true, state_data).await?;
                let expanded_if_false = Self::expand_variables(if_false, state_data).await?;
                Ok(PipelineStep::Condition {
                    name: name.clone(),
                    condition: expanded_condition,
                    if_true: expanded_if_true,
                    if_false: expanded_if_false,
                })
            }
            PipelineStep::PrintOutput { name, value } => {
                let expanded_value = Self::expand_variables(value, state_data).await?;
                Ok(PipelineStep::PrintOutput {
                    name: name.clone(),
                    value: expanded_value,
                })
            }
            // For other step types, we can simply clone them as they are
            _ => Ok(step.clone()),
        }
    }

    /// Basic variable expansion
    pub async fn expand_variables(
        input: &str,
        state_data: &HashMap<String, String>,
    ) -> Result<String, Error> {
        debug!("Expanding variables in input: {}", input);
        let mut result = input.to_string();
        
        // Expand ${VAR} format
        for (key, value) in state_data {
            result = result.replace(&format!("${{{}}}", key), value);
        }
        
        Ok(result)
    }

    /// Advanced variable expansion with multiple formats
    pub async fn expand_variables_advanced(
        input: &str,
        state_data: &HashMap<String, String>,
    ) -> Result<String, Error> {
        debug!("Expanding variables (advanced) in input: {}", input);
        let mut result = input.to_string();
        
        // Expand ${VAR} format
        for (key, value) in state_data {
            result = result.replace(&format!("${{{}}}", key), value);
            result = result.replace(&format!("${}", key), value); // Also support $VAR format
        }
        
        // Handle environment variables
        result = Self::expand_environment_variables(&result)?;
        
        Ok(result)
    }

    /// Expand environment variables (simple implementation)
    fn expand_environment_variables(input: &str) -> Result<String, Error> {
        let mut result = input.to_string();

        // Simple pattern matching for $ENV{VAR}
        while let Some(start) = result.find("$ENV{") {
            if let Some(end) = result[start..].find('}') {
                let env_var = &result[start + 5..start + end];
                let env_value = std::env::var(env_var).unwrap_or_default();
                result.replace_range(start..start + end + 1, &env_value);
            } else {
                break;
            }
        }

        Ok(result)
    }

    /// Expand variables with nested expansion
    pub async fn expand_variables_nested(
        input: &str,
        state_data: &HashMap<String, String>,
        max_depth: usize,
    ) -> Result<String, Error> {
        debug!("Expanding variables (nested) in input: {}", input);
        let mut result = input.to_string();
        let mut depth = 0;
        
        while depth < max_depth {
            let previous_result = result.clone();
            
            // Perform one round of expansion
            for (key, value) in state_data {
                result = result.replace(&format!("${{{}}}", key), value);
            }
            
            // If no changes were made, we're done
            if result == previous_result {
                break;
            }
            
            depth += 1;
        }
        
        if depth >= max_depth {
            debug!("Variable expansion reached maximum depth: {}", max_depth);
        }
        
        Ok(result)
    }

    /// Validate variable names (simple implementation)
    pub fn validate_variable_name(name: &str) -> bool {
        if name.is_empty() {
            return false;
        }

        // Safe to unwrap here since we already checked for empty string
        // But let's use a safer approach
        let first_char = match name.chars().next() {
            Some(c) => c,
            None => return false, // This should never happen due to empty check above
        };

        if !first_char.is_ascii_alphabetic() && first_char != '_' {
            return false;
        }

        name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    }

    /// Extract variable names from a string (simple implementation)
    pub fn extract_variable_names(input: &str) -> Vec<String> {
        let mut variables = Vec::new();
        let mut chars = input.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '$' && chars.peek() == Some(&'{') {
                chars.next(); // consume '{'
                let mut var_name = String::new();

                while let Some(ch) = chars.next() {
                    if ch == '}' {
                        break;
                    }
                    var_name.push(ch);
                }

                if !var_name.is_empty() {
                    variables.push(var_name);
                }
            }
        }

        variables
    }

    /// Check if a string contains variables
    pub fn contains_variables(input: &str) -> bool {
        input.contains("${") || input.contains("$ENV{")
    }
}
