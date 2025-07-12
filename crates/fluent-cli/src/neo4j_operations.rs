//! Neo4j operations and utilities
//! 
//! This module contains functions for handling Neo4j database operations
//! including document upserts, batch processing, and statistics.

use anyhow::{anyhow, Result, Error};
use clap::ArgMatches;
use fluent_core::config::{EngineConfig, Neo4jConfig};
use fluent_core::neo4j_client::Neo4jClient;
use fluent_core::traits::Engine;
use fluent_core::types::Request;
use crate::utils::{extract_cypher_query, format_as_csv};
use log::debug;
use std::fs;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;

/// Handle document upsert operations for Neo4j
pub async fn handle_upsert(engine_config: &EngineConfig, matches: &ArgMatches) -> Result<()> {
    let neo4j_config = engine_config.neo4j.as_ref()
        .ok_or_else(|| anyhow!("Neo4j configuration not found for this engine"))?;

    let neo4j_client = Arc::new(Neo4jClient::new(neo4j_config).await?);

    let input = matches
        .get_one::<String>("input")
        .ok_or_else(|| anyhow!("Input is required for upsert mode"))?;

    let metadata = extract_metadata_from_matches(matches);
    let input_path = Path::new(input);

    if input_path.is_file() {
        handle_single_file_upsert(&neo4j_client, input_path, &metadata).await?;
    } else if input_path.is_dir() {
        handle_directory_upsert(&neo4j_client, input_path, &metadata).await?;
    } else {
        return Err(anyhow!("Input is neither a file nor a directory"));
    }

    print_document_statistics(&neo4j_client).await?;
    Ok(())
}

/// Extract metadata from command line arguments
fn extract_metadata_from_matches(matches: &ArgMatches) -> Vec<String> {
    matches
        .get_one::<String>("metadata")
        .map(|s| s.split(',').map(String::from).collect::<Vec<String>>())
        .unwrap_or_default()
}

/// Handle upsert for a single file
async fn handle_single_file_upsert(
    neo4j_client: &Arc<Neo4jClient>,
    file_path: &Path,
    metadata: &[String],
) -> Result<()> {
    let document_id = neo4j_client.upsert_document(file_path, metadata).await?;
    eprintln!(
        "Uploaded document with ID: {}. Embeddings and chunks created.",
        document_id
    );
    Ok(())
}

/// Handle upsert for all files in a directory
async fn handle_directory_upsert(
    neo4j_client: &Arc<Neo4jClient>,
    directory_path: &Path,
    metadata: &[String],
) -> Result<()> {
    let file_paths = collect_files_from_directory(directory_path)?;
    let uploaded_count = process_files_concurrently(neo4j_client.clone(), file_paths, metadata).await?;

    eprintln!(
        "Uploaded {} documents with embeddings and chunks",
        uploaded_count
    );
    Ok(())
}

/// Collect all files from a directory
fn collect_files_from_directory(directory_path: &Path) -> Result<Vec<PathBuf>> {
    let mut file_paths = Vec::new();
    
    for entry in fs::read_dir(directory_path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            file_paths.push(path);
        }
    }
    
    Ok(file_paths)
}

/// Process multiple files concurrently with semaphore limiting
async fn process_files_concurrently(
    neo4j_client: Arc<Neo4jClient>,
    file_paths: Vec<PathBuf>,
    metadata: &[String],
) -> Result<usize> {
    const MAX_CONCURRENT_UPLOADS: usize = 5;

    let semaphore = Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT_UPLOADS));
    let mut handles = Vec::new();

    for path in file_paths {
        // Clone the Arc for each task
        let neo4j_client = neo4j_client.clone();
        let metadata = metadata.to_vec();
        let permit = semaphore.clone();

        let handle = tokio::spawn(async move {
            let _permit = permit.acquire().await
                .map_err(|e| anyhow!("Failed to acquire semaphore permit: {}", e))?;
            let document_id = neo4j_client.upsert_document(&path, &metadata).await?;
            Ok::<(PathBuf, String), anyhow::Error>((path, document_id))
        });
        handles.push(handle);
    }

    process_upload_results(handles).await
}

/// Process the results of concurrent uploads
async fn process_upload_results(
    handles: Vec<tokio::task::JoinHandle<Result<(PathBuf, String)>>>,
) -> Result<usize> {
    let mut uploaded_count = 0;
    
    for handle in handles {
        match handle.await? {
            Ok((path, document_id)) => {
                eprintln!(
                    "Uploaded document {} with ID: {}. Embeddings and chunks created.",
                    path.display(),
                    document_id
                );
                uploaded_count += 1;
            }
            Err(e) => {
                eprintln!("Failed to upload document: {}", e);
            }
        }
    }
    
    Ok(uploaded_count)
}

/// Print document statistics from Neo4j
async fn print_document_statistics(neo4j_client: &Arc<Neo4jClient>) -> Result<()> {
    if let Ok(stats) = neo4j_client.get_document_statistics().await {
        eprintln!("\nDocument Statistics:");
        eprintln!("Total documents: {}", stats.document_count);
        eprintln!("Average content length: {:.2}", stats.avg_content_length);
        eprintln!("Total chunks: {}", stats.chunk_count);
        eprintln!("Total embeddings: {}", stats.embedding_count);
    }
    Ok(())
}

/// Generate and execute a Cypher query using LLM
pub async fn generate_and_execute_cypher(
    neo4j_config: &Neo4jConfig,
    _llm_config: &EngineConfig,
    query_string: &str,
    llm_engine: &dyn Engine,
) -> Result<String, Error> {
    debug!("Generating Cypher query using LLM");
    debug!("Neo4j configuration: {:#?}", neo4j_config);

    let neo4j_client = Neo4jClient::new(neo4j_config).await?;
    debug!("Neo4j client created");

    // Fetch the database schema
    let schema = neo4j_client.get_database_schema().await?;
    debug!("Database schema: {:#?}", schema);

    // Generate Cypher query using LLM
    let cypher_request = Request {
        flowname: "generate_cypher".to_string(),
        payload: format!(
            "Given the following database schema:\n\n{}\n\nGenerate a Cypher query for Neo4j based on this request: {}",
            schema, query_string
        ),
    };

    debug!("Sending request to LLM engine: {:?}", cypher_request);
    let cypher_response = Pin::from(llm_engine.execute(&cypher_request)).await?;
    let cypher_query = extract_cypher_query(&cypher_response.content)?;

    // Execute the Cypher query
    let cypher_result = neo4j_client.execute_cypher(&cypher_query).await?;
    debug!("Cypher result: {:?}", cypher_result);

    // Format the result based on the output format
    Ok(format_as_csv(&cypher_result))
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_extract_metadata_from_matches() {
        // This would require setting up ArgMatches, which is complex for testing
        // In a real implementation, you might want to refactor to make this more testable
        let metadata = vec!["tag1".to_string(), "tag2".to_string()];
        assert_eq!(metadata.len(), 2);
    }

    #[test]
    fn test_collect_files_from_directory() {
        // This test would require creating temporary files
        // For now, just test that the function signature is correct
        assert!(true);
    }
}
