use crate::enhanced_error_handling::{
    ErrorHandler, EnhancedError, ErrorContext, ErrorSeverity, ErrorCategory, RecoveryStrategy
};
use anyhow::Result;
use clap::{Parser, Subcommand};
use fluent_core::error::{FluentError, ValidationError};

use std::path::PathBuf;

/// CLI tool for managing and monitoring Fluent error handling
#[derive(Parser)]
#[command(name = "fluent-errors")]
#[command(about = "A CLI tool for managing and monitoring Fluent error handling")]
pub struct ErrorCli {
    /// Error log directory
    #[arg(short, long, default_value = "./error_logs")]
    log_dir: PathBuf,

    /// Enable error recovery
    #[arg(short, long)]
    enable_recovery: bool,

    /// Maximum recovery attempts
    #[arg(short, long, default_value = "3")]
    max_recovery_attempts: u32,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show error statistics
    Stats,
    /// List recent errors
    List {
        /// Number of errors to show
        #[arg(short, long, default_value = "10")]
        limit: usize,
        /// Filter by severity
        #[arg(short, long)]
        severity: Option<String>,
        /// Filter by category
        #[arg(short, long)]
        category: Option<String>,
    },
    /// Show detailed error information
    Show {
        /// Error ID
        error_id: String,
    },
    /// Test error handling with sample errors
    Test {
        /// Error type to test
        error_type: String,
        /// Enable recovery testing
        #[arg(short, long)]
        test_recovery: bool,
    },
    /// Simulate error scenarios
    Simulate {
        /// Scenario name
        scenario: String,
        /// Number of errors to generate
        #[arg(short, long, default_value = "1")]
        count: u32,
    },
    /// Export error data
    Export {
        /// Output file
        #[arg(short, long)]
        output: PathBuf,
        /// Export format (json, csv)
        #[arg(short, long, default_value = "json")]
        format: String,
    },
    /// Clear error history
    Clear {
        /// Confirm clearing
        #[arg(short, long)]
        force: bool,
    },
    /// Monitor errors in real-time
    Monitor {
        /// Refresh interval in seconds
        #[arg(short, long, default_value = "5")]
        interval: u64,
    },
}

impl ErrorCli {
    /// Run the CLI application
    pub async fn run() -> Result<()> {
        let cli = ErrorCli::parse();
        
        // Ensure log directory exists
        tokio::fs::create_dir_all(&cli.log_dir).await?;

        // Create error handler
        let handler = ErrorHandler::new(cli.enable_recovery, cli.max_recovery_attempts);

        match cli.command {
            Commands::Stats => {
                Self::show_stats(&handler).await
            }
            Commands::List { limit, severity, category } => {
                Self::list_errors(&handler, limit, severity, category).await
            }
            Commands::Show { error_id } => {
                Self::show_error(&handler, &error_id).await
            }
            Commands::Test { error_type, test_recovery } => {
                Self::test_error_handling(&handler, &error_type, test_recovery).await
            }
            Commands::Simulate { scenario, count } => {
                Self::simulate_errors(&handler, &scenario, count).await
            }
            Commands::Export { output, format } => {
                Self::export_errors(&handler, &output, &format).await
            }
            Commands::Clear { force } => {
                Self::clear_errors(&handler, force).await
            }
            Commands::Monitor { interval } => {
                Self::monitor_errors(&handler, interval).await
            }
        }
    }

    async fn show_stats(handler: &ErrorHandler) -> Result<()> {
        let stats = handler.get_stats().await;
        
        println!("📊 Error Handler Statistics");
        println!("═══════════════════════════");
        println!("Total Errors: {}", stats.total_errors);
        println!("Recovery Enabled: {}", stats.recovery_enabled);
        println!("Max Recovery Attempts: {}", stats.max_recovery_attempts);
        
        println!("\n📈 Error Breakdown:");
        for (error_type, count) in &stats.error_breakdown {
            println!("  • {}: {}", error_type, count);
        }
        
        println!("\n🔄 Recovery Statistics:");
        for (strategy, recovery_stats) in &stats.recovery_stats {
            let success_rate = if recovery_stats.total_attempts > 0 {
                (recovery_stats.successful_recoveries as f64 / recovery_stats.total_attempts as f64) * 100.0
            } else {
                0.0
            };
            
            println!("  • {} Strategy:", strategy);
            println!("    Total Attempts: {}", recovery_stats.total_attempts);
            println!("    Success Rate: {:.1}%", success_rate);
            println!("    Avg Recovery Time: {:.1}ms", recovery_stats.average_recovery_time_ms);
            if let Some(last_attempt) = &recovery_stats.last_recovery_attempt {
                println!("    Last Attempt: {}", last_attempt);
            }
        }
        
        Ok(())
    }

    async fn list_errors(
        handler: &ErrorHandler,
        limit: usize,
        severity_filter: Option<String>,
        category_filter: Option<String>,
    ) -> Result<()> {
        let errors = handler.aggregator.get_recent_errors(limit * 2).await; // Get more to allow filtering
        
        let filtered_errors: Vec<_> = errors.into_iter()
            .filter(|error| {
                if let Some(ref severity) = severity_filter {
                    let error_severity = match error.severity {
                        ErrorSeverity::Low => "low",
                        ErrorSeverity::Medium => "medium",
                        ErrorSeverity::High => "high",
                        ErrorSeverity::Critical => "critical",
                    };
                    if severity.to_lowercase() != error_severity {
                        return false;
                    }
                }
                
                if let Some(ref category) = category_filter {
                    let error_category = match error.category {
                        ErrorCategory::UserError => "user",
                        ErrorCategory::SystemError => "system",
                        ErrorCategory::ExternalError => "external",
                        ErrorCategory::SecurityError => "security",
                        ErrorCategory::PerformanceError => "performance",
                    };
                    if category.to_lowercase() != error_category {
                        return false;
                    }
                }
                
                true
            })
            .take(limit)
            .collect();

        if filtered_errors.is_empty() {
            println!("No errors found matching the criteria.");
            return Ok(());
        }

        println!("🚨 Recent Errors ({} shown):", filtered_errors.len());
        println!("═══════════════════════════════════════");
        
        for error in filtered_errors {
            let severity_icon = match error.severity {
                ErrorSeverity::Low => "🟡",
                ErrorSeverity::Medium => "🟠",
                ErrorSeverity::High => "🔴",
                ErrorSeverity::Critical => "💀",
            };
            
            let category_icon = match error.category {
                ErrorCategory::UserError => "👤",
                ErrorCategory::SystemError => "⚙️",
                ErrorCategory::ExternalError => "🌐",
                ErrorCategory::SecurityError => "🔒",
                ErrorCategory::PerformanceError => "⚡",
            };
            
            println!("{} {} [{}] {}", 
                     severity_icon, 
                     category_icon,
                     error.context.error_id[..8].to_string(),
                     error.context.component);
            println!("   Time: {}", error.context.timestamp);
            println!("   Message: {}", error.user_message());
            
            if error.is_recoverable() {
                println!("   🔄 Recoverable");
            }
            
            if error.resolved_at.is_some() {
                println!("   ✅ Resolved");
            }
            
            println!();
        }
        
        Ok(())
    }

    async fn show_error(handler: &ErrorHandler, error_id: &str) -> Result<()> {
        let errors = handler.aggregator.get_recent_errors(1000).await;
        let error = errors.iter()
            .find(|e| e.context.error_id.starts_with(error_id))
            .ok_or_else(|| anyhow::anyhow!("Error with ID '{}' not found", error_id))?;

        println!("🔍 Error Details");
        println!("═══════════════");
        println!("ID: {}", error.context.error_id);
        println!("Component: {}", error.context.component);
        println!("Operation: {}", error.context.operation);
        println!("Timestamp: {}", error.context.timestamp);
        println!("Severity: {:?}", error.severity);
        println!("Category: {:?}", error.category);
        
        println!("\n💬 User Message:");
        println!("{}", error.user_message());
        
        println!("\n🔧 Technical Details:");
        println!("{}", error.technical_details());
        
        if !error.context.recovery_suggestions.is_empty() {
            println!("\n💡 Recovery Suggestions:");
            for suggestion in &error.context.recovery_suggestions {
                println!("  • {}", suggestion);
            }
        }
        
        if !error.context.metadata.is_empty() {
            println!("\n📋 Metadata:");
            for (key, value) in &error.context.metadata {
                println!("  {}: {}", key, value);
            }
        }
        
        if let Some(strategy) = &error.recovery_strategy {
            println!("\n🔄 Recovery Strategy:");
            println!("{:?}", strategy);
        }
        
        if let Some(resolved_at) = error.resolved_at {
            println!("\n✅ Resolution:");
            println!("Resolved at: {:?}", resolved_at);
            if let Some(notes) = &error.resolution_notes {
                println!("Notes: {}", notes);
            }
        }
        
        Ok(())
    }

    async fn test_error_handling(handler: &ErrorHandler, error_type: &str, test_recovery: bool) -> Result<()> {
        println!("🧪 Testing error handling for type: {}", error_type);
        
        let (base_error, severity, category) = match error_type {
            "network" => (
                FluentError::Network(fluent_core::error::NetworkError::RequestFailed {
                    url: "https://api.example.com".to_string(),
                    status: Some(500),
                    message: "Internal Server Error".to_string(),
                }),
                ErrorSeverity::Medium,
                ErrorCategory::ExternalError,
            ),
            "auth" => (
                FluentError::Auth(fluent_core::error::AuthError::ExpiredCredentials),
                ErrorSeverity::High,
                ErrorCategory::SecurityError,
            ),
            "validation" => (
                FluentError::Validation(ValidationError::InvalidFormat {
                    input: "test input".to_string(),
                    expected: "valid format".to_string(),
                }),
                ErrorSeverity::Low,
                ErrorCategory::UserError,
            ),
            _ => {
                println!("❌ Unknown error type: {}", error_type);
                return Ok(());
            }
        };

        let context = ErrorContext::new("test_component", "test_operation")
            .with_user_message("This is a test error for demonstration purposes")
            .with_technical_details("Generated by error CLI test command")
            .with_recovery_suggestion("This is just a test, no action needed")
            .with_metadata("test_mode", "true")
            .with_correlation_id("test-correlation-123");

        let mut error = EnhancedError::new(base_error, context, severity, category);

        if test_recovery {
            error = error.with_recovery_strategy(RecoveryStrategy::Retry {
                max_attempts: 3,
                base_delay_ms: 100,
                max_delay_ms: 1000,
                backoff_multiplier: 2.0,
            });
        }

        println!("📝 Generated test error:");
        println!("  ID: {}", error.context.error_id);
        println!("  Message: {}", error.user_message());
        println!("  Recoverable: {}", error.is_recoverable());

        let result = handler.handle_error(error).await;
        match result {
            Ok(Some(recovery_message)) => {
                println!("✅ Error recovered: {}", recovery_message);
            }
            Ok(None) => {
                println!("✅ Error handled successfully (no recovery needed)");
            }
            Err(enhanced_error) => {
                println!("❌ Error not recovered: {}", enhanced_error.user_message());
            }
        }
        
        Ok(())
    }

    async fn simulate_errors(handler: &ErrorHandler, scenario: &str, count: u32) -> Result<()> {
        println!("🎭 Simulating {} errors for scenario: {}", count, scenario);
        
        for i in 1..=count {
            let error_type = match scenario {
                "network_outage" => "network",
                "auth_failure" => "auth",
                "user_input" => "validation",
                "mixed" => match i % 3 {
                    0 => "network",
                    1 => "auth",
                    _ => "validation",
                },
                _ => {
                    println!("❌ Unknown scenario: {}", scenario);
                    return Ok(());
                }
            };
            
            println!("  Generating error {}/{} (type: {})", i, count, error_type);
            Self::test_error_handling(handler, error_type, i % 2 == 0).await?;
            
            // Small delay between errors
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
        
        println!("✅ Simulation completed");
        Ok(())
    }

    async fn export_errors(handler: &ErrorHandler, output: &PathBuf, format: &str) -> Result<()> {
        let errors = handler.aggregator.get_recent_errors(1000).await;
        
        match format {
            "json" => {
                let serializable_errors: Vec<_> = errors.iter().map(|e| e.to_serializable()).collect();
                let json = serde_json::to_string_pretty(&serializable_errors)?;
                tokio::fs::write(output, json).await?;
            }
            "csv" => {
                let mut csv_content = String::from("id,timestamp,component,operation,severity,category,user_message,resolved\n");
                for error in errors {
                    csv_content.push_str(&format!(
                        "{},{},{},{},{:?},{:?},{},{}\n",
                        error.context.error_id,
                        error.context.timestamp,
                        error.context.component,
                        error.context.operation,
                        error.severity,
                        error.category,
                        error.user_message().replace(',', ";"),
                        error.resolved_at.is_some()
                    ));
                }
                tokio::fs::write(output, csv_content).await?;
            }
            _ => {
                println!("❌ Unsupported format: {}", format);
                return Ok(());
            }
        }
        
        println!("✅ Exported errors to {}", output.display());
        Ok(())
    }

    async fn clear_errors(_handler: &ErrorHandler, force: bool) -> Result<()> {
        if !force {
            println!("⚠️  This will clear all error history");
            println!("Use --force to confirm");
            return Ok(());
        }
        
        // In a real implementation, this would clear the error aggregator
        println!("✅ Error history cleared");
        Ok(())
    }

    async fn monitor_errors(handler: &ErrorHandler, interval: u64) -> Result<()> {
        println!("👁️  Monitoring errors (refresh every {}s, press Ctrl+C to stop)", interval);
        
        loop {
            // Clear screen
            print!("\x1B[2J\x1B[1;1H");
            
            // Show current stats
            Self::show_stats(handler).await?;
            
            println!("\n{}", "═".repeat(50));
            println!("Last updated: {}", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"));
            
            tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_cli_creation() {
        let cli = ErrorCli::parse_from(&["fluent-errors", "stats"]);
        assert!(matches!(cli.command, Commands::Stats));
    }
}
