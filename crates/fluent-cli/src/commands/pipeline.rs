use anyhow::{anyhow, Result};
use clap::ArgMatches;
use fluent_core::config::Config;
use fluent_core::error::FluentResult;
use fluent_engines::pipeline_executor::{FileStateStore, Pipeline, PipelineExecutor, StateStore};
use std::env;
use std::path::PathBuf;

use super::{CommandHandler, CommandResult};

/// Pipeline command handler
pub struct PipelineCommand;

impl PipelineCommand {
    pub fn new() -> Self {
        Self
    }

    /// Validate pipeline YAML content
    fn validate_pipeline_yaml(yaml_content: &str) -> FluentResult<()> {
        // Basic YAML structure validation
        if yaml_content.trim().is_empty() {
            return Err(fluent_core::error::FluentError::Validation(
                fluent_core::error::ValidationError::MissingField(
                    "Pipeline YAML content is empty".to_string(),
                ),
            ));
        }

        // Validate YAML syntax
        match serde_yaml::from_str::<serde_yaml::Value>(yaml_content) {
            Ok(_) => Ok(()),
            Err(e) => Err(fluent_core::error::FluentError::Validation(
                fluent_core::error::ValidationError::InvalidFormat {
                    input: "YAML content".to_string(),
                    expected: format!("Valid YAML syntax: {}", e),
                },
            )),
        }
    }

    /// Get or create state store directory
    fn get_state_store_dir() -> Result<PathBuf> {
        let state_store_dir = match env::var("FLUENT_STATE_STORE") {
            Ok(path) => PathBuf::from(path),
            Err(_) => {
                eprintln!(
                    "Warning: FLUENT_STATE_STORE environment variable not set. Using default path."
                );
                PathBuf::from("./pipeline_states")
            }
        };
        Ok(state_store_dir)
    }

    /// Execute pipeline with the given parameters
    async fn execute_pipeline(
        pipeline_file: &str,
        input: &str,
        force_fresh: bool,
        run_id: Option<String>,
        json_output: bool,
    ) -> Result<CommandResult> {
        // Read and validate pipeline file
        let yaml_str = tokio::fs::read_to_string(pipeline_file)
            .await
            .map_err(|e| anyhow!("Failed to read pipeline file '{}': {}", pipeline_file, e))?;

        Self::validate_pipeline_yaml(&yaml_str)
            .map_err(|e| anyhow!("Pipeline validation failed: {}", e))?;

        let pipeline: Pipeline = serde_yaml::from_str(&yaml_str)
            .map_err(|e| anyhow!("Failed to parse pipeline YAML: {}", e))?;

        // Setup state store
        let state_store_dir = Self::get_state_store_dir()?;
        tokio::fs::create_dir_all(&state_store_dir)
            .await
            .map_err(|e| anyhow!("Failed to create state store directory: {}", e))?;

        let state_store = FileStateStore {
            directory: state_store_dir.clone(),
        };

        // Create and execute pipeline
        let executor = PipelineExecutor::new(state_store, json_output);
        executor
            .execute(&pipeline, input, force_fresh, run_id.clone())
            .await
            .map_err(|e| anyhow!("Pipeline execution failed: {}", e))?;

        // Handle output
        if json_output {
            let state_key = format!(
                "{}-{}",
                pipeline.name,
                run_id.unwrap_or_else(|| "unknown".to_string())
            );

            // Create a new state store for loading (since the original was moved)
            let load_state_store = FileStateStore {
                directory: state_store_dir,
            };

            if let Some(state) = load_state_store.load_state(&state_key).await? {
                let json_output = serde_json::to_string_pretty(&state)
                    .map_err(|e| anyhow!("Failed to serialize state: {}", e))?;
                println!("{}", json_output);
                Ok(CommandResult::success_with_data(
                    serde_json::to_value(state)
                        .map_err(|e| anyhow!("Failed to serialize state to JSON: {}", e))?,
                ))
            } else {
                let error_msg = "No state file found for the given run ID.";
                eprintln!("{}", error_msg);
                Ok(CommandResult::error(error_msg.to_string()))
            }
        } else {
            Ok(CommandResult::success_with_message(format!(
                "Pipeline '{}' executed successfully",
                pipeline.name
            )))
        }
    }
}

impl CommandHandler for PipelineCommand {
    async fn execute(&self, matches: &ArgMatches, _config: &Config) -> Result<()> {
        // Extract pipeline arguments
        let pipeline_file = matches
            .get_one::<String>("file")
            .ok_or_else(|| anyhow!("Pipeline file is required"))?;

        let input = matches
            .get_one::<String>("input")
            .ok_or_else(|| anyhow!("Pipeline input is required"))?;

        let force_fresh = matches.get_flag("force_fresh");
        let run_id = matches.get_one::<String>("run_id").cloned();
        let json_output = matches.get_flag("json_output");

        // Execute pipeline
        let result =
            Self::execute_pipeline(pipeline_file, input, force_fresh, run_id, json_output).await?;

        if !result.success {
            if let Some(message) = result.message {
                return Err(anyhow!("Pipeline execution failed: {}", message));
            } else {
                return Err(anyhow!("Pipeline execution failed"));
            }
        }

        Ok(())
    }
}

impl Default for PipelineCommand {
    fn default() -> Self {
        Self::new()
    }
}
