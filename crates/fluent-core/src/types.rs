// crates/fluent-core/src/types.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Request {
    pub flowname: String,
    pub payload: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Response {
    pub content: String,
    pub usage: Usage,
    pub model: String,
    pub finish_reason: Option<String>,
    pub cost: Cost,

}


#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Cost {
    pub prompt_cost: f64,
    pub completion_cost: f64,
    pub total_cost: f64,
}



#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UpsertRequest {
    pub input: String,
    pub output: String,
    pub metadata: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UpsertResponse {
    pub processed_files: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct DocumentStatistics {
    pub document_count: i64,
    pub avg_content_length: f64,
    pub chunk_count: i64,
    pub embedding_count: i64,
}


#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ExtractedContent {
    pub main_content: String,
    pub sentiment: Option<String>,
    pub clusters: Option<Vec<String>>,
    pub themes: Option<Vec<String>>,
    pub keywords: Option<Vec<String>>,
}