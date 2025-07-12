//! Neo4j client modules
//! 
//! This module contains all the refactored Neo4j client functionality
//! organized into focused, single-responsibility modules.

pub mod document_processor;
pub mod enrichment;
pub mod interaction_manager;
pub mod query_executor;

// Re-export commonly used types and functions
pub use document_processor::{ChunkEmbeddingManager, DocumentExtractor, DocumentUpsertManager};
pub use enrichment::{DocumentEnrichmentManager, EnrichmentConfig, SentimentAnalysis};
pub use interaction_manager::{
    InteractionManager, InteractionStats, Neo4jInteraction, Neo4jSession,
};
pub use query_executor::{QueryExecutor, ResultProcessor, VerificationResult};
