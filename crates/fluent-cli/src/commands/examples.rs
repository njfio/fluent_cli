//! Examples command handler
//!
//! This module provides the command handler for showing detailed examples
//! for various FluentCLI commands.

use crate::commands::CommandHandler;
use anyhow::Result;
use clap::ArgMatches;
use fluent_core::config::Config;

/// Examples command handler
pub struct ExamplesCommand;

impl ExamplesCommand {
    /// Create a new examples command handler
    pub fn new() -> Self {
        Self
    }

    /// Show examples for a specific command
    fn show_command_examples(&self, command: &str) -> Result<()> {
        match command {
            "agent" => self.show_agent_examples(),
            "pipeline" => self.show_pipeline_examples(),
            "tools" => self.show_tools_examples(),
            "setup" => self.show_setup_examples(),
            "configure" => self.show_configure_examples(),
            "engine" => self.show_engine_examples(),
            "mcp" => self.show_mcp_examples(),
            "neo4j" => self.show_neo4j_examples(),
            "completions" => self.show_completions_examples(),
            "memory" => self.show_memory_examples(),
            _ => {
                println!("Unknown command: {}", command);
                println!("Available commands: agent, pipeline, tools, setup, configure, engine, mcp, neo4j, completions, memory");
                Ok(())
            }
        }
    }

    /// Show all examples
    fn show_all_examples(&self) -> Result<()> {
        println!("🌟 FluentCLI Comprehensive Examples");
        println!("===================================\n");
        
        self.show_agent_examples()?;
        println!("\n{}\n", "=".repeat(50));
        
        self.show_pipeline_examples()?;
        println!("\n{}\n", "=".repeat(50));
        
        self.show_tools_examples()?;
        println!("\n{}\n", "=".repeat(50));
        
        self.show_setup_examples()?;
        println!("\n{}\n", "=".repeat(50));
        
        self.show_configure_examples()?;
        println!("\n{}\n", "=".repeat(50));
        
        self.show_engine_examples()?;
        println!("\n{}\n", "=".repeat(50));
        
        self.show_mcp_examples()?;
        println!("\n{}\n", "=".repeat(50));
        
        self.show_neo4j_examples()?;
        println!("\n{}\n", "=".repeat(50));
        
        self.show_completions_examples()?;
        println!("\n{}\n", "=".repeat(50));
        
        self.show_memory_examples()?;
        
        Ok(())
    }

    fn show_agent_examples(&self) -> Result<()> {
        println!("🤖 Agent Command Examples");
        println!("==========================");
        println!();
        println!("📝 Basic Usage:");
        println!("  fluent agent \"Hello, world!\"");
        println!("  fluent agent \"Write a Rust function to calculate fibonacci numbers\"");
        println!("  fluent agent \"Analyze this code for performance issues\"");
        println!();
        println!("🔧 Advanced Usage:");
        println!("  fluent agent \"Refactor this legacy code\" --interactive");
        println!("  fluent agent \"Debug this error message\" --verbose");
        println!("  fluent agent \"Create a comprehensive test suite\" --config custom.toml");
        println!("  fluent agent \"Build a REST API\" --enable-tools --max-iterations 20");
        println!();
        println!("🎯 Common Tasks:");
        println!("  fluent agent \"Generate API documentation\"");
        println!("  fluent agent \"Optimize database queries\"");
        println!("  fluent agent \"Implement authentication middleware\"");
        println!("  fluent agent \"Create deployment scripts\"");
        println!("  fluent agent \"Write unit tests for this function\"");
        println!();
        println!("💡 Tips:");
        println!("  • Use quotes around your goal/task");
        println!("  • --interactive enables human-in-the-loop control");
        println!("  • --enable-tools allows file and system operations");
        println!("  • --tui provides a better monitoring experience");
        println!("  • Use --reflection for complex multi-step tasks");
        Ok(())
    }

    fn show_pipeline_examples(&self) -> Result<()> {
        println!("🔄 Pipeline Command Examples");
        println!("============================");
        println!();
        println!("📝 Basic Usage:");
        println!("  fluent pipeline -f example_pipelines/test_pipeline.yaml");
        println!("  fluent pipeline -f pipelines/code_review.yaml -i \"Review this PR\"");
        println!("  fluent pipeline -f workflow.yaml --variables API_KEY=xxx");
        println!();
        println!("🔧 Advanced Usage:");
        println!("  fluent pipeline -f complex_workflow.yaml --verbose");
        println!("  fluent pipeline -f pipelines/ci_cd.yaml --config production.toml");
        println!("  fluent pipeline -f pipeline.yaml --dry-run");
        println!("  fluent pipeline -f workflow.yaml --variables KEY1=val1 --variables KEY2=val2");
        println!();
        println!("📋 Available Example Pipelines:");
        println!("  • example_pipelines/test_pipeline.yaml - Basic agent execution");
        println!("  • example_pipelines/code_analysis.yaml - Code review workflow");
        println!("  • example_pipelines/documentation.yaml - Documentation generation");
        println!();
        println!("💡 Tips:");
        println!("  • Use --dry-run to preview execution");
        println!("  • Multiple --variables flags are supported");
        println!("  • Pipeline files must be valid YAML");
        Ok(())
    }

    fn show_tools_examples(&self) -> Result<()> {
        println!("🛠️  Tools Command Examples");
        println!("==========================");
        println!();
        println!("📝 List Tools:");
        println!("  fluent tools list");
        println!("  fluent tools list --category file");
        println!("  fluent tools list --category network");
        println!("  fluent tools list --available");
        println!("  fluent tools list --detailed");
        println!();
        println!("🔍 Search and Describe:");
        println!("  fluent tools list --search \"file\"");
        println!("  fluent tools describe read_file --schema --examples");
        println!("  fluent tools describe write_file --examples");
        println!();
        println!("⚙️  Execute Tools:");
        println!("  fluent tools exec read_file --json '{{\"path\": \"README.md\"}}'");
        println!("  fluent tools exec write_file --json '{{\"path\": \"test.txt\", \"content\": \"Hello\"}}'");
        println!();
        println!("📚 Categories:");
        println!("  fluent tools categories");
        println!();
        println!("💡 Tips:");
        println!("  • Use --json for programmatic access");
        println!("  • Categories: file, network, system, data, ai");
        println!("  • --available shows only enabled tools");
        Ok(())
    }

    fn show_setup_examples(&self) -> Result<()> {
        println!("🚀 Setup Command Examples");
        println!("==========================");
        println!();
        println!("📝 Basic Setup:");
        println!("  fluent setup");
        println!("  fluent setup --output my_config.toml");
        println!();
        println!("🔧 Advanced Setup:");
        println!("  fluent setup --force --skip-validation");
        println!("  fluent setup -o production_config.toml");
        println!();
        println!("💡 Pro Tips:");
        println!("  • Set API keys in environment variables for auto-detection");
        println!("  • Supported: ANTHROPIC_API_KEY, OPENAI_API_KEY, etc.");
        println!("  • Use --force to overwrite existing configurations");
        println!("  • Run setup first to configure your AI engines");
        Ok(())
    }

    fn show_configure_examples(&self) -> Result<()> {
        println!("⚙️  Configure Command Examples");
        println!("==============================");
        println!();
        println!("📝 View Configuration:");
        println!("  fluent configure show");
        println!("  fluent configure show --json");
        println!();
        println!("🎯 Presets:");
        println!("  fluent configure presets");
        println!("  fluent configure presets --apply developer");
        println!("  fluent configure presets --apply production");
        println!("  fluent configure presets --apply researcher");
        println!();
        println!("🔧 Optimize:");
        println!("  fluent configure optimize --dry-run");
        println!("  fluent configure optimize");
        println!();
        println!("⚡ Set Values:");
        println!("  fluent configure set memory.max_tokens 8000");
        println!("  fluent configure set agent.max_iterations 20");
        Ok(())
    }

    fn show_engine_examples(&self) -> Result<()> {
        println!("⚙️  Engine Command Examples");
        println!("============================");
        println!();
        println!("📝 List Engines:");
        println!("  fluent engine list");
        println!("  fluent engine list --json");
        println!();
        println!("🧪 Test Engines:");
        println!("  fluent engine test anthropic");
        println!("  fluent engine test openai");
        println!("  fluent engine test groq --verbose");
        println!();
        println!("💡 Tips:");
        println!("  • Test engines after configuration");
        println!("  • Use --json for programmatic access");
        println!("  • Engine names are case-sensitive");
        Ok(())
    }

    fn show_mcp_examples(&self) -> Result<()> {
        println!("🔌 MCP Command Examples");
        println!("=======================");
        println!();
        println!("📝 Server:");
        println!("  fluent mcp server");
        println!("  fluent mcp server --port 8080");
        println!("  fluent mcp server --port 9090");
        println!();
        println!("🔗 Client:");
        println!("  fluent mcp client --server http://localhost:8080");
        println!("  fluent mcp client --server https://example.com:8080");
        println!();
        println!("💡 Tips:");
        println!("  • Default port is 8080");
        println!("  • URL must include protocol");
        Ok(())
    }

    fn show_neo4j_examples(&self) -> Result<()> {
        println!("📊 Neo4j Command Examples");
        println!("========================");
        println!();
        println!("📝 Query:");
        println!("  fluent neo4j --query \"MATCH (n) RETURN n LIMIT 10\"");
        println!("  fluent neo4j --query \"Find all users\" --generate-cypher");
        println!();
        println!("📤 Upsert:");
        println!("  fluent neo4j --upsert-file users.json");
        println!("  fluent neo4j --upsert-file data.json");
        println!();
        println!("💡 Tips:");
        println!("  • Use --generate-cypher for natural language");
        println!("  • File format should be JSON");
        Ok(())
    }

    fn show_completions_examples(&self) -> Result<()> {
        println!("⌨️  Completions Command Examples");
        println!("===============================");
        println!();
        println!("📝 Generate Completions:");
        println!("  fluent completions --shell bash");
        println!("  fluent completions --shell zsh --output fluent.zsh");
        println!("  fluent completions --shell fish");
        println!("  fluent completions --shell powershell");
        println!();
        println!("💡 Tips:");
        println!("  • Supported: bash, zsh, fish, powershell, elvish");
        println!("  • Save to file for permanent installation");
        println!("  • Source in your shell's rc file");
        Ok(())
    }

    fn show_memory_examples(&self) -> Result<()> {
        println!("🧠 Memory Command Examples");
        println!("==========================");
        println!();
        println!("📝 View Insights:");
        println!("  fluent memory insights");
        println!("  fluent memory insights --limit 20");
        println!("  fluent memory insights --json");
        println!();
        println!("🔍 View Patterns:");
        println!("  fluent memory patterns");
        println!("  fluent memory patterns --domain programming");
        println!("  fluent memory patterns --domain file_management --json");
        println!();
        println!("📊 View Statistics:");
        println!("  fluent memory stats");
        println!("  fluent memory stats --json");
        println!();
        println!("💡 Tips:");
        println!("  • Run agent tasks first to generate learning data");
        println!("  • Insights show what the agent has learned");
        println!("  • Patterns show reusable successful approaches");
        Ok(())
    }
}

impl CommandHandler for ExamplesCommand {
    async fn execute(&self, matches: &ArgMatches, _config: &Config) -> Result<()> {
        if matches.get_flag("all") {
            self.show_all_examples()?;
        } else if let Some(command) = matches.get_one::<String>("command") {
            self.show_command_examples(command)?;
        } else {
            // Show available commands
            println!("🌟 FluentCLI Examples");
            println!("=====================\n");
            println!("Usage:");
            println!("  fluent examples <command>   # Show examples for a command");
            println!("  fluent examples --all       # Show all examples\n");
            println!("Available commands:");
            println!("  • agent       - Agentic workflows");
            println!("  • pipeline    - Pipeline execution");
            println!("  • tools       - Tool management");
            println!("  • setup       - Initial setup");
            println!("  • configure   - Configuration management");
            println!("  • engine      - Engine management");
            println!("  • mcp         - MCP operations");
            println!("  • neo4j       - Neo4j operations");
            println!("  • completions - Shell completions");
            println!("  • memory      - Memory insights and patterns");
        }
        Ok(())
    }
}

