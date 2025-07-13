use crate::modular_pipeline_executor::{ExecutionContext, Pipeline, PipelineStep, RetryConfig};
use crate::pipeline_infrastructure::PipelineExecutorBuilder;
use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use fluent_core::centralized_config::ConfigManager;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

use std::time::Duration;

/// CLI tool for managing and executing Fluent pipelines
#[derive(Parser)]
#[command(name = "fluent-pipeline")]
#[command(about = "A CLI tool for managing and executing Fluent pipelines")]
pub struct PipelineCli {
    /// Pipeline directory
    #[arg(short, long)]
    pipeline_dir: Option<PathBuf>,

    /// State directory for execution context
    #[arg(short, long)]
    state_dir: Option<PathBuf>,

    /// Log directory for execution logs
    #[arg(short, long)]
    log_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all available pipelines
    List,
    /// Show pipeline details
    Show {
        /// Pipeline name
        name: String,
    },
    /// Execute a pipeline
    Execute {
        /// Pipeline name
        name: String,
        /// Initial variables in KEY=VALUE format
        #[arg(short, long)]
        var: Vec<String>,
        /// Resume from existing run ID
        #[arg(short, long)]
        resume: Option<String>,
    },
    /// Validate a pipeline
    Validate {
        /// Pipeline name
        name: String,
    },
    /// Create a new pipeline template
    Create {
        /// Pipeline name
        name: String,
        /// Pipeline description
        #[arg(short, long)]
        description: Option<String>,
    },
    /// Show execution metrics
    Metrics,
    /// Show execution history
    History {
        /// Number of executions to show
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
    /// Monitor pipeline execution in real-time
    Monitor {
        /// Run ID to monitor
        run_id: String,
        /// Refresh interval in seconds
        #[arg(short, long, default_value = "2")]
        interval: u64,
    },
    /// Cancel a running pipeline
    Cancel {
        /// Run ID to cancel
        run_id: String,
    },
}

impl PipelineCli {
    /// Run the CLI application
    pub async fn run() -> Result<()> {
        // Initialize centralized configuration
        ConfigManager::initialize()?;
        let config = ConfigManager::get();

        let cli = PipelineCli::parse();

        // Get directories from CLI args or use centralized config defaults
        let pipeline_dir = cli.pipeline_dir.unwrap_or_else(|| config.get_pipeline_dir());
        let state_dir = cli.state_dir.unwrap_or_else(|| config.get_pipeline_state_dir());
        let log_dir = cli.log_dir.unwrap_or_else(|| config.paths.pipeline_logs_directory.clone());

        // Ensure directories exist
        tokio::fs::create_dir_all(&pipeline_dir).await?;
        tokio::fs::create_dir_all(&state_dir).await?;
        tokio::fs::create_dir_all(&log_dir).await?;

        match cli.command {
            Commands::List => Self::list_pipelines(&pipeline_dir).await,
            Commands::Show { name } => Self::show_pipeline(&pipeline_dir, &name).await,
            Commands::Execute {
                ref name,
                ref var,
                ref resume,
            } => Self::execute_pipeline(&pipeline_dir, &state_dir, name, var.clone(), resume.clone()).await,
            Commands::Validate { name } => Self::validate_pipeline(&pipeline_dir, &name).await,
            Commands::Create { name, description } => {
                Self::create_pipeline(&pipeline_dir, &name, description.as_deref()).await
            }
            Commands::Metrics => Self::show_metrics(&state_dir).await,
            Commands::History { limit } => Self::show_history(&state_dir, limit).await,
            Commands::Monitor { run_id, interval } => {
                Self::monitor_execution(&state_dir, &run_id, interval).await
            }
            Commands::Cancel { run_id } => Self::cancel_execution(&run_id).await,
        }
    }

    async fn list_pipelines(pipeline_dir: &PathBuf) -> Result<()> {
        let mut entries = tokio::fs::read_dir(pipeline_dir).await?;
        let mut pipelines = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            if let Some(file_name) = entry.file_name().to_str() {
                if file_name.ends_with(".json") {
                    let pipeline_name = file_name.trim_end_matches(".json");
                    pipelines.push(pipeline_name.to_string());
                }
            }
        }

        if pipelines.is_empty() {
            println!("No pipelines found in {}", pipeline_dir.display());
            return Ok(());
        }

        println!("🔧 Available pipelines:");
        for pipeline_name in pipelines {
            // Try to load pipeline to get description
            match Self::load_pipeline(pipeline_dir, &pipeline_name).await {
                Ok(pipeline) => {
                    let description = pipeline
                        .description
                        .unwrap_or_else(|| "No description".to_string());
                    println!("  • {} - {}", pipeline_name, description);
                    println!(
                        "    Version: {}, Steps: {}",
                        pipeline.version,
                        pipeline.steps.len()
                    );
                }
                Err(_) => {
                    println!("  • {} (failed to load)", pipeline_name);
                }
            }
        }

        Ok(())
    }

    async fn show_pipeline(pipeline_dir: &PathBuf, name: &str) -> Result<()> {
        let pipeline = Self::load_pipeline(pipeline_dir, name).await?;

        println!("🔧 Pipeline: {}", pipeline.name);
        println!("Version: {}", pipeline.version);
        if let Some(description) = &pipeline.description {
            println!("Description: {}", description);
        }

        if let Some(timeout) = &pipeline.timeout {
            println!("Timeout: {:?}", timeout);
        }

        if let Some(max_parallel) = pipeline.max_parallel {
            println!("Max Parallel: {}", max_parallel);
        }

        println!("\n📋 Steps ({}):", pipeline.steps.len());
        for (i, step) in pipeline.steps.iter().enumerate() {
            println!("  {}. {} ({})", i + 1, step.name, step.step_type);

            if !step.depends_on.is_empty() {
                println!("     Depends on: {:?}", step.depends_on);
            }

            if let Some(condition) = &step.condition {
                println!("     Condition: {}", condition);
            }

            if let Some(retry_config) = &step.retry_config {
                println!("     Retry: {} attempts", retry_config.max_attempts);
            }

            if let Some(timeout) = &step.timeout {
                println!("     Timeout: {:?}", timeout);
            }
        }

        if !pipeline.global_config.is_empty() {
            println!("\n⚙️  Global Config:");
            for (key, value) in &pipeline.global_config {
                println!("  {}: {}", key, value);
            }
        }

        Ok(())
    }

    async fn execute_pipeline(
        pipeline_dir: &PathBuf,
        state_dir: &PathBuf,
        name: &str,
        variables: Vec<String>,
        resume: Option<String>,
    ) -> Result<()> {
        let pipeline = Self::load_pipeline(pipeline_dir, name).await?;

        // Parse variables
        let mut initial_variables = HashMap::new();
        for var in variables {
            let parts: Vec<&str> = var.splitn(2, '=').collect();
            if parts.len() == 2 {
                initial_variables.insert(parts[0].to_string(), parts[1].to_string());
            } else {
                return Err(anyhow!("Invalid variable format: '{}'. Use KEY=VALUE", var));
            }
        }

        // Create executor with metrics
        let log_file = pipeline_dir.join("logs").join(format!("{}.log", name));
        if let Some(parent_dir) = log_file.parent() {
            tokio::fs::create_dir_all(parent_dir).await?;
        } else {
            return Err(anyhow!("Invalid log file path: {}", log_file.display()));
        }

        let (builder, metrics_listener) = PipelineExecutorBuilder::new()
            .with_file_state_store(state_dir.clone())
            .with_simple_variable_expander()
            .with_console_logging()
            .with_file_logging(log_file)
            .with_metrics();

        let executor = builder.build()?;

        println!("🚀 Executing pipeline: {}", name);
        if let Some(resume_id) = &resume {
            println!("📄 Resuming from run ID: {}", resume_id);
        }

        let start_time = std::time::Instant::now();

        match executor
            .execute_pipeline(&pipeline, initial_variables, resume)
            .await
        {
            Ok(context) => {
                let duration = start_time.elapsed();
                println!("✅ Pipeline completed successfully!");
                println!("   Run ID: {}", context.run_id);
                println!("   Duration: {:?}", duration);
                println!("   Steps executed: {}", context.step_history.len());

                // Show final variables
                if !context.variables.is_empty() {
                    println!("\n📝 Final variables:");
                    for (key, value) in &context.variables {
                        println!("   {}: {}", key, value);
                    }
                }

                // Show metrics
                let metrics = metrics_listener.get_metrics().await;
                println!("\n📊 Execution metrics:");
                println!("   Total steps: {}", metrics.total_steps);
                println!("   Successful steps: {}", metrics.successful_steps);
                println!("   Failed steps: {}", metrics.failed_steps);
            }
            Err(e) => {
                let duration = start_time.elapsed();
                println!("❌ Pipeline failed!");
                println!("   Duration: {:?}", duration);
                println!("   Error: {}", e);

                // Show metrics even on failure
                let metrics = metrics_listener.get_metrics().await;
                if metrics.total_steps > 0 {
                    println!("\n📊 Execution metrics:");
                    println!("   Total steps: {}", metrics.total_steps);
                    println!("   Successful steps: {}", metrics.successful_steps);
                    println!("   Failed steps: {}", metrics.failed_steps);
                }

                return Err(e);
            }
        }

        Ok(())
    }

    async fn validate_pipeline(pipeline_dir: &PathBuf, name: &str) -> Result<()> {
        println!("🔍 Validating pipeline: {}", name);

        let pipeline = Self::load_pipeline(pipeline_dir, name).await?;

        // Basic validation
        if pipeline.steps.is_empty() {
            return Err(anyhow!("Pipeline has no steps"));
        }

        // Check for duplicate step names
        let mut step_names = std::collections::HashSet::new();
        for step in &pipeline.steps {
            if !step_names.insert(&step.name) {
                return Err(anyhow!("Duplicate step name: {}", step.name));
            }
        }

        // Check dependencies
        for step in &pipeline.steps {
            for dep in &step.depends_on {
                if !step_names.contains(dep) {
                    return Err(anyhow!(
                        "Step '{}' depends on non-existent step '{}'",
                        step.name,
                        dep
                    ));
                }
            }
        }

        // Check for circular dependencies (simple check)
        for step in &pipeline.steps {
            if step.depends_on.contains(&step.name) {
                return Err(anyhow!("Step '{}' depends on itself", step.name));
            }
        }

        println!("✅ Pipeline validation successful!");
        println!("   Steps: {}", pipeline.steps.len());
        println!(
            "   Dependencies: {}",
            pipeline
                .steps
                .iter()
                .map(|s| s.depends_on.len())
                .sum::<usize>()
        );

        Ok(())
    }

    async fn create_pipeline(
        pipeline_dir: &PathBuf,
        name: &str,
        description: Option<&str>,
    ) -> Result<()> {
        let pipeline = Pipeline {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            description: description.map(String::from),
            steps: vec![
                PipelineStep {
                    name: "hello".to_string(),
                    step_type: "command".to_string(),
                    config: [
                        (
                            "command".to_string(),
                            Value::String("echo 'Hello, World!'".to_string()),
                        ),
                        (
                            "save_output".to_string(),
                            Value::String("greeting".to_string()),
                        ),
                    ]
                    .into_iter()
                    .collect(),
                    timeout: Some(Duration::from_secs(ConfigManager::get().pipeline.default_timeout_seconds)),
                    retry_config: Some(RetryConfig {
                        max_attempts: ConfigManager::get().pipeline.retry_attempts,
                        base_delay_ms: ConfigManager::get().pipeline.retry_base_delay_ms,
                        max_delay_ms: ConfigManager::get().pipeline.retry_max_delay_ms,
                        backoff_multiplier: ConfigManager::get().pipeline.retry_backoff_multiplier,
                        retry_on: vec!["timeout".to_string()],
                    }),
                    depends_on: Vec::new(),
                    condition: None,
                    parallel_group: None,
                },
                PipelineStep {
                    name: "goodbye".to_string(),
                    step_type: "command".to_string(),
                    config: [(
                        "command".to_string(),
                        Value::String("echo 'Goodbye from ${greeting}!'".to_string()),
                    )]
                    .into_iter()
                    .collect(),
                    timeout: Some(Duration::from_secs(ConfigManager::get().pipeline.default_timeout_seconds)),
                    retry_config: None,
                    depends_on: vec!["hello".to_string()],
                    condition: None,
                    parallel_group: None,
                },
            ],
            global_config: [
                (
                    "timeout".to_string(),
                    Value::Number(serde_json::Number::from(ConfigManager::get().pipeline.default_timeout_seconds)),
                ),
                (
                    "max_parallel".to_string(),
                    Value::Number(serde_json::Number::from(ConfigManager::get().pipeline.max_parallel_steps)),
                ),
            ]
            .into_iter()
            .collect(),
            timeout: Some(Duration::from_secs(ConfigManager::get().pipeline.default_timeout_seconds)),
            max_parallel: Some(ConfigManager::get().pipeline.max_parallel_steps),
        };

        let pipeline_file = pipeline_dir.join(format!("{}.json", name));
        let json = serde_json::to_string_pretty(&pipeline)?;
        tokio::fs::write(&pipeline_file, json).await?;

        println!("✅ Created pipeline template: {}", name);
        println!("   File: {}", pipeline_file.display());
        println!("   Steps: {}", pipeline.steps.len());

        println!("\n📝 Next steps:");
        println!("  1. Edit the pipeline file to customize steps");
        println!("  2. Validate: fluent-pipeline validate {}", name);
        println!("  3. Execute: fluent-pipeline execute {}", name);

        Ok(())
    }

    async fn show_metrics(_state_dir: &PathBuf) -> Result<()> {
        println!("📊 Pipeline execution metrics:");
        println!("   [Metrics display not yet implemented]");
        println!("   This would show aggregated metrics across all pipeline executions");

        Ok(())
    }

    async fn show_history(state_dir: &PathBuf, limit: usize) -> Result<()> {
        println!("📜 Pipeline execution history (last {}):", limit);

        let mut entries = tokio::fs::read_dir(state_dir).await?;
        let mut contexts = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            if let Some(file_name) = entry.file_name().to_str() {
                if file_name.ends_with(".json") {
                    if let Ok(content) = tokio::fs::read_to_string(entry.path()).await {
                        if let Ok(context) = serde_json::from_str::<ExecutionContext>(&content) {
                            contexts.push(context);
                        }
                    }
                }
            }
        }

        // Sort by start time (most recent first)
        contexts.sort_by(|a, b| b.start_time.cmp(&a.start_time));
        contexts.truncate(limit);

        if contexts.is_empty() {
            println!("   No execution history found");
            return Ok(());
        }

        for context in contexts {
            let status =
                if context
                    .step_history
                    .iter()
                    .any(|s| s.status == crate::modular_pipeline_executor::ExecutionStatus::Failed)
                {
                    "❌ Failed"
                } else if context.step_history.iter().all(|s| {
                    s.status == crate::modular_pipeline_executor::ExecutionStatus::Completed
                }) {
                    "✅ Completed"
                } else {
                    "🔄 In Progress"
                };

            println!(
                "  • {} - {} ({})",
                context.run_id[..8].to_string(),
                context.pipeline_name,
                status
            );
            println!("    Started: {:?}", context.start_time);
            println!(
                "    Steps: {}/{}",
                context.step_history.len(),
                context.current_step
            );
        }

        Ok(())
    }

    async fn monitor_execution(state_dir: &PathBuf, run_id: &str, interval: u64) -> Result<()> {
        println!(
            "👁️  Monitoring execution: {} (refresh every {}s)",
            run_id, interval
        );

        loop {
            let state_file = state_dir.join(format!("{}.json", run_id));

            if !state_file.exists() {
                println!("❌ Execution not found: {}", run_id);
                break;
            }

            let content = tokio::fs::read_to_string(&state_file).await?;
            let context: ExecutionContext = serde_json::from_str(&content)?;

            // Clear screen
            print!("\x1B[2J\x1B[1;1H");

            println!("🔄 Pipeline Execution Monitor");
            println!("═══════════════════════════════");
            println!("Run ID: {}", context.run_id);
            println!("Pipeline: {}", context.pipeline_name);
            println!("Started: {:?}", context.start_time);
            println!("Current Step: {}", context.current_step);

            println!("\n📋 Step History:");
            for (i, step) in context.step_history.iter().enumerate() {
                let status_icon = match step.status {
                    crate::modular_pipeline_executor::ExecutionStatus::Pending => "⏳",
                    crate::modular_pipeline_executor::ExecutionStatus::Running => "🔄",
                    crate::modular_pipeline_executor::ExecutionStatus::Completed => "✅",
                    crate::modular_pipeline_executor::ExecutionStatus::Failed => "❌",
                    crate::modular_pipeline_executor::ExecutionStatus::Skipped => "⏭️",
                    crate::modular_pipeline_executor::ExecutionStatus::Cancelled => "🚫",
                };

                println!(
                    "  {}. {} {} ({})",
                    i + 1,
                    status_icon,
                    step.step_name,
                    step.step_type
                );

                if let Some(error) = &step.error {
                    println!("     Error: {}", error);
                }

                if step.retry_count > 0 {
                    println!("     Retries: {}", step.retry_count);
                }
            }

            if !context.variables.is_empty() {
                println!("\n📝 Variables:");
                for (key, value) in &context.variables {
                    println!("  {}: {}", key, value);
                }
            }

            println!("\n{}", "═".repeat(50));
            println!(
                "Last updated: {}",
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
            );
            println!("Press Ctrl+C to stop monitoring");

            tokio::time::sleep(Duration::from_secs(interval)).await;
        }

        Ok(())
    }

    async fn cancel_execution(run_id: &str) -> Result<()> {
        println!("🚫 Cancelling execution: {}", run_id);
        println!("   [Cancellation not yet implemented]");
        println!("   This would send a cancellation signal to the running pipeline");

        Ok(())
    }

    async fn load_pipeline(pipeline_dir: &PathBuf, name: &str) -> Result<Pipeline> {
        let pipeline_file = pipeline_dir.join(format!("{}.json", name));

        if !pipeline_file.exists() {
            return Err(anyhow!("Pipeline '{}' not found", name));
        }

        let content = tokio::fs::read_to_string(&pipeline_file).await?;
        let pipeline: Pipeline = serde_json::from_str(&content)?;

        Ok(pipeline)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_cli_creation() {
        let cli = PipelineCli::parse_from(&["fluent-pipeline", "list"]);
        assert!(matches!(cli.command, Commands::List));
    }
}
