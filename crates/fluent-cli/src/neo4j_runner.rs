//! Neo4j operations and Cypher query generation
//!
//! This module provides functionality for Neo4j database operations,
//! including Cypher query generation using LLMs.

use anyhow::Result;
use fluent_core::config::{Config, EngineConfig};
use fluent_core::traits::Engine;
use fluent_core::types::Request;

/// Get Neo4j query LLM engine from configuration
pub async fn get_neo4j_query_llm(config: &Config) -> Option<(Box<dyn Engine>, &EngineConfig)> {
    let neo4j_config = config.engines.iter().find(|e| e.engine == "neo4j")?;
    let query_llm = neo4j_config.neo4j.as_ref()?.query_llm.as_ref()?;
    let llm_config = config.engines.iter().find(|e| e.name == *query_llm)?;
    let engine = crate::create_engine(llm_config).await.ok()?;
    Some((engine, llm_config))
}

/// Generate Cypher query from natural language using LLM
pub async fn generate_cypher_query(query: &str, config: &EngineConfig) -> Result<String> {
    // Use the configured LLM to generate a Cypher query
    let llm_request = Request {
        flowname: "cypher_generation".to_string(),
        payload: format!(
            "Convert this natural language query to Cypher: {query}. \
            Only return the Cypher query, no explanations."
        ),
    };

    let engine = crate::create_engine(config).await?;
    let response = std::pin::Pin::from(engine.execute(&llm_request)).await?;
    
    // Extract just the Cypher query from the response
    let cypher = response.content.trim();
    
    // Basic validation - ensure it looks like a Cypher query
    if !cypher.to_uppercase().contains("MATCH") && 
       !cypher.to_uppercase().contains("CREATE") && 
       !cypher.to_uppercase().contains("MERGE") {
        return Err(anyhow::anyhow!("Generated query doesn't appear to be valid Cypher: {}", cypher));
    }
    
    Ok(cypher.to_string())
}
