use anyhow::{anyhow, Result};
use clap::ArgMatches;
use fluent_core::config::{Config, EngineConfig};
use fluent_core::neo4j_client::Neo4jClient;
use fluent_core::traits::Engine;
use fluent_core::types::Request;
use fluent_engines::create_engine;

use super::{CommandHandler, CommandResult};

/// Neo4j command handler for graph database operations
pub struct Neo4jCommand;

impl Neo4jCommand {
    pub fn new() -> Self {
        Self
    }

    /// Get Neo4j query LLM engine
    async fn get_neo4j_query_llm(config: &Config) -> Option<(Box<dyn Engine>, &EngineConfig)> {
        let neo4j_config = config.engines.iter().find(|e| e.engine == "neo4j")?;
        let query_llm = neo4j_config.neo4j.as_ref()?.query_llm.as_ref()?;
        let llm_config = config.engines.iter().find(|e| e.name == *query_llm)?;
        let engine = create_engine(llm_config).await.ok()?;
        Some((engine, llm_config))
    }

    /// Generate Cypher query using LLM
    async fn generate_cypher_query(query: &str, config: &EngineConfig) -> Result<String> {
        let engine = create_engine(config).await?;

        let llm_request = Request {
            flowname: "cypher_generation".to_string(),
            payload: format!(
                "Generate a Cypher query for Neo4j based on this request: {}",
                query
            ),
        };

        let response = std::pin::Pin::from(engine.execute(&llm_request)).await?;

        // Extract Cypher query from response (simplified)
        let cypher = response
            .content
            .lines()
            .find(|line| line.trim().starts_with("MATCH") || line.trim().starts_with("CREATE"))
            .unwrap_or(&response.content)
            .trim()
            .to_string();

        Ok(cypher)
    }

    /// Execute Cypher query generation
    async fn execute_cypher_generation(query: &str, config: &Config) -> Result<CommandResult> {
        println!("🔍 Generating Cypher query for: {}", query);

        // Get Neo4j configuration and query LLM
        let (_llm_engine, llm_config) = Self::get_neo4j_query_llm(config)
            .await
            .ok_or_else(|| anyhow!("Neo4j configuration or query LLM not found"))?;

        // Generate Cypher query
        let cypher_query = Self::generate_cypher_query(query, llm_config).await?;

        println!("📝 Generated Cypher query:");
        println!("{}", cypher_query);

        // Find Neo4j engine configuration
        let neo4j_config = config
            .engines
            .iter()
            .find(|e| e.engine == "neo4j")
            .ok_or_else(|| anyhow!("Neo4j engine configuration not found"))?;

        // Execute the query if Neo4j client is available
        if let Some(neo4j_settings) = &neo4j_config.neo4j {
            println!("🔗 Connecting to Neo4j...");

            let neo4j_client = Neo4jClient::new(neo4j_settings).await?;

            println!("⚡ Executing Cypher query...");
            let results = neo4j_client.execute_cypher(&cypher_query).await?;

            println!("📊 Query results:");
            println!("  {}", results);

            Ok(CommandResult::success_with_data(serde_json::json!({
                "query": query,
                "cypher": cypher_query,
                "results": results
            })))
        } else {
            println!("⚠️  Neo4j client not configured, showing generated query only");

            Ok(CommandResult::success_with_data(serde_json::json!({
                "query": query,
                "cypher": cypher_query
            })))
        }
    }

    /// Execute Neo4j upsert operation
    async fn execute_upsert_operation(
        input_path: &str,
        metadata_terms: Option<&str>,
        config: &Config,
    ) -> Result<CommandResult> {
        println!("📤 Starting Neo4j upsert operation");
        println!("Input: {}", input_path);

        if let Some(terms) = metadata_terms {
            println!("Metadata terms: {}", terms);
        }

        // Find Neo4j engine configuration
        let neo4j_config = config
            .engines
            .iter()
            .find(|e| e.engine == "neo4j")
            .ok_or_else(|| anyhow!("Neo4j engine configuration not found"))?;

        let neo4j_settings = neo4j_config
            .neo4j
            .as_ref()
            .ok_or_else(|| anyhow!("Neo4j settings not found in configuration"))?;

        // Connect to Neo4j
        let neo4j_client = Neo4jClient::new(neo4j_settings).await?;

        // Process input (simplified implementation)
        let content = tokio::fs::read_to_string(input_path).await
            .map_err(|e| anyhow!("Failed to read input file '{}': {}", input_path, e))?;

        println!("📝 Processing content ({} characters)", content.len());

        // Create upsert query (simplified)
        let upsert_query = format!(
            "MERGE (d:Document {{path: '{}'}}) SET d.content = $content, d.updated = timestamp()",
            input_path
        );

        // Execute upsert
        println!("⚡ Executing upsert...");
        let results = neo4j_client.execute_cypher(&upsert_query).await?;

        println!("✅ Upsert completed successfully");
        println!("📊 Results: {}", results);

        Ok(CommandResult::success_with_data(serde_json::json!({
            "input_path": input_path,
            "metadata_terms": metadata_terms,
            "records_affected": 1
        })))
    }
}

impl CommandHandler for Neo4jCommand {
    async fn execute(&self, matches: &ArgMatches, config: &Config) -> Result<()> {
        if let Some(cypher_query) = matches.get_one::<String>("generate-cypher") {
            // Generate and execute Cypher query
            let result = Self::execute_cypher_generation(cypher_query, config).await?;

            if !result.success {
                if let Some(message) = result.message {
                    return Err(anyhow!("Cypher generation failed: {}", message));
                } else {
                    return Err(anyhow!("Cypher generation failed"));
                }
            }
        } else if matches.get_flag("upsert") {
            // Execute upsert operation
            let input_path = matches
                .get_one::<String>("input")
                .ok_or_else(|| anyhow!("Input path is required for upsert operation"))?;

            let metadata_terms = matches.get_one::<String>("metadata");

            let result = Self::execute_upsert_operation(
                input_path,
                metadata_terms.map(|s| s.as_str()),
                config,
            )
            .await?;

            if !result.success {
                if let Some(message) = result.message {
                    eprintln!("Upsert operation failed: {}", message);
                }
                std::process::exit(1);
            }
        } else {
            return Err(anyhow!(
                "No Neo4j operation specified. Use --generate-cypher or --upsert"
            ));
        }

        Ok(())
    }
}

impl Default for Neo4jCommand {
    fn default() -> Self {
        Self::new()
    }
}
