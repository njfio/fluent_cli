use reqwest::Client;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use crate::neo4j_client::VoyageAIConfig;


pub const EMBEDDING_DIMENSION: usize = 1536;

#[derive(Serialize)]
struct VoyageAIRequest {
    input: Vec<String>,
    model: String,
}

#[derive(Deserialize)]
struct VoyageAIResponse {
    data: Vec<VoyageAIEmbedding>,
}

#[derive(Deserialize)]
struct VoyageAIEmbedding {
    embedding: Vec<f32>,
}

pub async fn get_voyage_embedding(text: &str, config: &VoyageAIConfig) -> Result<Vec<f32>> {
    let client = Client::new();

    let request_body = VoyageAIRequest {
        input: vec![text.to_string()],
        model: config.model.clone(),
    };

    let response: VoyageAIResponse = client.post("https://api.voyageai.com/v1/embeddings")
        .header("Authorization", format!("Bearer {}", config.api_key))
        .json(&request_body)
        .send()
        .await?
        .json()
        .await?;

    Ok(response.data[0].embedding.clone())
}

