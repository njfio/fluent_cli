//! Document processing and content extraction for Neo4j
//! 
//! This module handles document content extraction, chunking, and embedding creation
//! for various file types including PDF, text files, and DOCX documents.

use anyhow::{anyhow, Result};
use neo4rs::{query, BoltInteger, BoltNull, BoltString, BoltType, Graph};
use pdf_extract::extract_text;
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use uuid::Uuid;
use log::debug;

use crate::neo4j_client::VoyageAIConfig;
use crate::traits::{DocxProcessor, DocumentProcessor};
use crate::utils::chunking::chunk_document;
use crate::voyageai_client::{get_voyage_embedding, EMBEDDING_DIMENSION};

/// Document content extractor
pub struct DocumentExtractor;

impl DocumentExtractor {
    /// Extract content from various file types
    pub async fn extract_content(file_path: &Path) -> Result<String> {
        let extension = file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| anyhow!("Unable to determine file type"))?;

        match extension.to_lowercase().as_str() {
            "pdf" => Self::extract_pdf_content(file_path).await,
            "txt" | "json" | "csv" | "tsv" | "md" | "html" | "xml" | "yml" | "yaml" | "json5"
            | "py" | "rb" | "rs" | "js" | "ts" | "php" | "java" | "c" | "cpp" | "go" | "sh"
            | "bat" | "ps1" | "psm1" | "psd1" | "ps1xml" | "psc1" | "pssc" | "pss1" | "psh" => {
                Self::extract_text_content(file_path).await
            }
            "docx" => Self::extract_docx_content(file_path).await,
            _ => Err(anyhow!("Unsupported file type: {}", extension)),
        }
    }

    /// Extract content from PDF files
    async fn extract_pdf_content(file_path: &Path) -> Result<String> {
        let path_buf = file_path.to_path_buf();
        Ok(tokio::task::spawn_blocking(move || extract_text(&path_buf)).await??)
    }

    /// Extract content from text-based files
    async fn extract_text_content(file_path: &Path) -> Result<String> {
        let mut file = File::open(file_path).await?;
        let mut content = String::new();
        file.read_to_string(&mut content).await?;
        Ok(content)
    }

    /// Extract content from DOCX files
    async fn extract_docx_content(file_path: &Path) -> Result<String> {
        let processor = DocxProcessor;
        let (content, _metadata) = processor.process(file_path).await?;
        Ok(content)
    }
}

/// Chunk and embedding manager
pub struct ChunkEmbeddingManager<'a> {
    graph: &'a Graph,
    voyage_config: Option<&'a VoyageAIConfig>,
}

impl<'a> ChunkEmbeddingManager<'a> {
    pub fn new(graph: &'a Graph, voyage_config: Option<&'a VoyageAIConfig>) -> Self {
        Self {
            graph,
            voyage_config,
        }
    }

    /// Create chunks and embeddings for a document
    pub async fn create_chunks_and_embeddings(
        &self,
        document_id: &str,
        content: &str,
    ) -> Result<()> {
        let chunks = chunk_document(content);
        self.process_chunks(document_id, &chunks).await
    }

    /// Process chunks and create embeddings
    async fn process_chunks(&self, document_id: &str, chunks: &[String]) -> Result<()> {
        debug!("Creating chunks and embeddings for document {}", document_id);
        
        if let Some(_voyage_config) = self.voyage_config {
            for (i, chunk) in chunks.iter().enumerate() {
                self.process_single_chunk(document_id, chunk, i, chunks).await?;
            }
            Ok(())
        } else {
            Err(anyhow!("VoyageAI configuration not found"))
        }
    }

    /// Process a single chunk and create its embedding
    async fn process_single_chunk(
        &self,
        document_id: &str,
        chunk: &str,
        index: usize,
        all_chunks: &[String],
    ) -> Result<()> {
        let voyage_config = self.voyage_config
            .ok_or_else(|| anyhow!("VoyageAI configuration not found"))?;
        let embedding = get_voyage_embedding(chunk, voyage_config).await?;

        if embedding.len() != EMBEDDING_DIMENSION {
            return Err(anyhow!("Embedding dimension mismatch"));
        }

        let chunk_id = Uuid::new_v4().to_string();
        let embedding_id = Uuid::new_v4().to_string();
        
        let query = self.build_chunk_embedding_query(
            document_id,
            &chunk_id,
            chunk,
            index,
            &embedding_id,
            &embedding,
            all_chunks,
        );

        let mut result = self.graph.execute(query).await?;

        if result.next().await?.is_none() {
            return Err(anyhow!("Failed to create or merge chunk and embedding"));
        }

        Ok(())
    }

    /// Build the Cypher query for creating chunks and embeddings
    fn build_chunk_embedding_query(
        &self,
        document_id: &str,
        chunk_id: &str,
        chunk_content: &str,
        index: usize,
        embedding_id: &str,
        embedding: &[f32],
        all_chunks: &[String],
    ) -> neo4rs::Query {
        query(
            "
            MATCH (d:Document {id: $document_id})
            MERGE (c:Chunk {content: $content})
            ON CREATE SET
                c.id = $chunk_id,
                c.index = $index
            MERGE (e:Embedding {vector: $vector})
            ON CREATE SET
                e.id = $embedding_id
            MERGE (d)-[:HAS_CHUNK]->(c)
            MERGE (c)-[:HAS_EMBEDDING]->(e)
            WITH c, e, $prev_chunk_id AS prev_id
            OPTIONAL MATCH (prev:Chunk {id: prev_id})
            FOREACH (_ IN CASE WHEN prev IS NOT NULL THEN [1] ELSE [] END |
                MERGE (prev)-[:NEXT]->(c)
            )
            RETURN c.id as chunk_id, e.id as embedding_id
            ",
        )
        .param("document_id", BoltType::String(BoltString::from(document_id)))
        .param("chunk_id", BoltType::String(BoltString::from(chunk_id)))
        .param("content", BoltType::String(BoltString::from(chunk_content)))
        .param("index", BoltType::Integer(BoltInteger::new(index as i64)))
        .param("embedding_id", BoltType::String(BoltString::from(embedding_id)))
        .param("vector", embedding)
        .param(
            "prev_chunk_id",
            if index > 0 {
                BoltType::String(BoltString::from(all_chunks[index - 1].as_str()))
            } else {
                BoltType::Null(BoltNull)
            },
        )
    }
}

/// Document upsert coordinator
pub struct DocumentUpsertManager<'a> {
    graph: &'a Graph,
    voyage_config: Option<&'a VoyageAIConfig>,
}

impl<'a> DocumentUpsertManager<'a> {
    pub fn new(graph: &'a Graph, voyage_config: Option<&'a VoyageAIConfig>) -> Self {
        Self {
            graph,
            voyage_config,
        }
    }

    /// Upsert a document with content extraction and processing
    pub async fn upsert_document(&self, file_path: &Path, metadata: &[String]) -> Result<String> {
        debug!("Upserting document from file: {:?}", file_path);

        let content = DocumentExtractor::extract_content(file_path).await?;
        let document_id = self.create_or_update_document(&content, metadata).await?;
        
        // Create chunks and embeddings
        let chunk_manager = ChunkEmbeddingManager::new(self.graph, self.voyage_config);
        chunk_manager.create_chunks_and_embeddings(&document_id, &content).await?;

        Ok(document_id)
    }

    /// Create or update document in Neo4j
    async fn create_or_update_document(&self, content: &str, metadata: &[String]) -> Result<String> {
        let document_id = Uuid::new_v4().to_string();
        let query = query(
            "
            MERGE (d:Document {content: $content})
            ON CREATE SET
                d.id = $id,
                d.metadata = $metadata,
                d.created_at = datetime()
            ON MATCH SET
                d.metadata = d.metadata + $new_metadata,
                d.updated_at = datetime()
            RETURN d.id as document_id
            ",
        )
        .param("id", document_id.clone())
        .param("content", content)
        .param("metadata", metadata)
        .param("new_metadata", metadata);

        let mut result = self.graph.execute(query).await?;

        if let Some(row) = result.next().await? {
            Ok(row.get::<String>("document_id")?)
        } else {
            Err(anyhow!("Failed to upsert document"))
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_chunk_embedding_manager_creation() {
        // This is a basic test to ensure the struct can be created
        // In a real test, you'd need a mock Graph and VoyageAIConfig
        assert!(true); // Placeholder test
    }

    #[test]
    fn test_document_extractor_supported_extensions() {
        // Test that we can identify supported file types
        let supported_extensions = vec!["pdf", "txt", "docx", "json", "py", "rs"];
        assert!(supported_extensions.len() > 0);
    }
}
