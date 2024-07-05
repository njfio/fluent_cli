use thiserror::Error;
use neo4rs::{DeError, Error as Neo4rsError};
use serde_json::Error as SerdeError;

#[derive(Error, Debug)]
pub enum Neo4jClientError {
    #[error("Neo4j error: {0}")]
    Neo4jError(#[from] Neo4rsError),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] SerdeError),

    #[error("Deserialization error: {0}")]
    DeserializationError(#[from] DeError),

    #[error("Vector representation error: {0}")]
    VectorRepresentationError(String),

    #[error("Other error: {0}")]
    Other(String),

    #[error("Other error: {0}")]
    OtherError(String),
}