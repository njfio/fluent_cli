use neo4rs::{Graph, query, Node as Neo4jNode, Relation, BoltString, BoltType, ConfigBuilder, Query, BoltBoolean, BoltFloat, BoltInteger, BoltMap, BoltList, BoltNull, DeError};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use uuid::Uuid;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::sync::{Arc, RwLock};
use log::{debug, error, info, warn};
use ndarray::{Array1, Array2};
use thiserror::Error;
use tokio::task;
use crate::config::FlowConfig;

use neo4rs::{ Error as Neo4rsError};
use serde_json::Error as SerdeError;
use stop_words::get;


use rust_stemmers::{Algorithm, Stemmer};
use rust_tokenizers::tokenizer::BaseTokenizer;
use tokio::io::{AsyncReadExt, AsyncWriteExt};


use rust_tokenizers::tokenizer::{Tokenizer};
use rustlearn::prelude::*;
use serde::de::StdError;
use crate::openai_agent_client::get_openai_embedding;


#[derive(Debug, Clone)]
pub enum LlmProvider {
    Anthropic,
    OpenAI,
    Google,
    Cohere,
    // Add other providers as needed
}
// Additional structs needed for the implementation
#[derive(Debug, Clone)]
pub struct Neo4jSession {
    pub id: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub context: String,
    pub session_id: String,
    pub user_id: String,
}

#[derive(Debug, Clone)]
pub struct Neo4jInteraction {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub order: i32,
    pub session_id: String,
    pub question: Option<Neo4jQuestion>,
    pub response: Option<Neo4jResponse>,
}

#[derive(Debug, Clone)]
pub struct Neo4jQuestion {
    pub id: String,
    pub content: String,
    pub vector: Vec<f32>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Neo4jResponse {
    pub id: String,
    pub content: String,
    pub vector: BoltList,
    pub timestamp: DateTime<Utc>,
    pub confidence: f64,
    pub llm_specific_data: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct Neo4jModel {
    pub id: String,
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone)]
pub struct Neo4jFlowConfiguration {
    pub id: String,
    pub config_hash: String,
    pub config_data: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct Neo4jTokenUsage {
    pub id: String,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
}

#[derive(Debug, Clone)]
pub struct Neo4jResponseMetrics {
    pub id: String,
    pub response_time: chrono::Duration,
    pub token_count: i32,
    pub confidence_score: f64,
}

#[derive(Debug, Clone)]
pub struct Neo4jModelPerformanceMetrics {
    pub model_name: String,
    pub response_count: i64,
    pub avg_response_time: chrono::Duration,
    pub avg_confidence_score: f64,
    pub total_tokens_used: i64,
}


pub struct Neo4jClient {
    graph: Graph,
    document_count: RwLock<usize>,
    word_document_count: RwLock<HashMap<String, usize>>,
}

#[derive(Debug, thiserror::Error)]
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

    #[error("OpenAI API error: {0}")]
    OpenAIError(#[from] Box<dyn StdError + Send + Sync>),

}






impl Neo4jClient {

    pub async fn initialize() -> Result<Self, Neo4jClientError> {
        debug!("Initializing Neo4j client...");
        let neo4j_uri = env::var("NEO4J_URI").expect("NEO4J_URI must be set");
        let neo4j_user = env::var("NEO4J_USER").expect("NEO4J_USER must be set");
        let neo4j_password = env::var("NEO4J_PASSWORD").expect("NEO4J_PASSWORD must be set");
        let neo4j_db = env::var("NEO4J_DB").expect("NEO4J_DB must be set");
        debug!("Connecting to Neo4j with URI: {}, user: {}, password: {}, and database: {}", neo4j_uri, neo4j_user, neo4j_password, neo4j_db);

        let config = ConfigBuilder::default()
            .uri(&neo4j_uri)
            .user(&neo4j_user)
            .password(&neo4j_password)
            .db(neo4j_db)
            .build()
            .map_err(Neo4jClientError::Neo4jError)?;

        let graph = Graph::connect(config).await?;

        // Initialize tokenizer
        let vocab_path = "/Users/n/RustroverProjects/fluent_cli/fluent_cli/vocab.json";
        let merges_path = "/Users/n/RustroverProjects/fluent_cli/fluent_cli/merges.txt";

        // let tokenizer = Arc::new(Self::create_tokenizer(vocab_path, merges_path)?);

        Ok(Neo4jClient {
            graph,
            document_count: RwLock::new(0),
            word_document_count: RwLock::new(HashMap::new()),
        })
    }

    pub async fn find_similar_themes(&self, vector: &[f32], limit: usize) -> Result<Vec<(String, f32)>, Neo4jClientError> {
        let query_str = r#"
    MATCH (t:Theme)
    WHERE t.vector IS NOT NULL
    WITH t, gds.similarity.cosine(t.vector, $vector) AS similarity
    ORDER BY similarity DESC
    LIMIT $limit
    RETURN t.value AS theme, similarity
    "#;

        let mut vector_list = BoltList::new();
        for &value in vector {
            vector_list.push(BoltType::Float(BoltFloat::new(value as f64)));
        }

        let mut result = self.graph.execute(query(query_str)
            .param("limit", BoltType::Integer(BoltInteger::new(limit as i64)))
            .param("vector", BoltType::List(vector_list))
        ).await?;

        let mut themes = Vec::new();

        while let Some(row) = result.next().await? {
            themes.push((
                row.get::<String>("theme")?,
                row.get::<f64>("similarity")? as f32
            ));
        }

        Ok(themes)
    }

    pub async fn create_or_update_session(&self, session: &Neo4jSession) -> Result<String, Neo4jClientError> {
        let query_str = r#"
    MERGE (s:Session {id: $id})
    ON CREATE SET
        s.start_time = $start_time,
        s.end_time = $end_time,
        s.context = $context,
        s.session_id = $session_id,
        s.user_id = $user_id
    ON MATCH SET
        s.end_time = $end_time,
        s.context = $context
    RETURN s.id as session_id
    "#;

        let query = query(query_str)
            .param("id", session.id.to_string())
            .param("start_time", session.start_time.to_rfc3339())
            .param("end_time", session.end_time.to_rfc3339())
            .param("context", session.context.to_string())
            .param("session_id", session.session_id.to_string())
            .param("user_id", session.user_id.to_string());

        debug!("Executing query for create_or_update_session");

        let mut result = self.graph.execute(query).await?;

        if let Some(row) = result.next().await? {
            let session_id: String = row.get("session_id")?;
            Ok(session_id)
        } else {
            Err(Neo4jClientError::Other("No result returned when creating session".to_string()))
        }
    }

    pub async fn create_interaction(&self, interaction: &Neo4jInteraction) -> Result<String, Neo4jClientError> {
        let query_str = r#"
    MATCH (s:Session {id: $session_id})
    CREATE (i:Interaction {
        id: $id,
        timestamp: $timestamp,
        order: $order
    })
    CREATE (s)-[:CONTAINS]->(i)
    RETURN i.id as interaction_id
    "#;

        let query = query(query_str)
            .param("id", interaction.id.to_string())
            .param("session_id", interaction.session_id.to_string())
            .param("timestamp", interaction.timestamp.to_rfc3339())
            .param("order", interaction.order);

        debug!("Executing query for create_interaction");
        debug!("Query: {}", query_str);
        debug!("Parameters: id={}, session_id={}, timestamp={}, order={}",
                 interaction.id, interaction.session_id, interaction.timestamp, interaction.order);

        let mut result = self.graph.execute(query).await?;

        if let Some(row) = result.next().await? {
            let interaction_id: String = row.get("interaction_id")?;
            Ok(interaction_id)
        } else {
            Err(Neo4jClientError::Other("No result returned when creating interaction node".to_string()))
        }
    }

    pub async fn create_or_update_question(&self, question: &Neo4jQuestion, interaction_id: &str) -> Result<String, Neo4jClientError> {
        let query_str = r#"
    MERGE (q:Question {content: $content})
    ON CREATE SET
        q = $props
    ON MATCH SET
        q.vector = $props.vector,
        q.timestamp = $props.timestamp
    WITH q
    MATCH (i:Interaction {id: $interaction_id})
    MERGE (i)-[:HAS_QUESTION]->(q)
    RETURN q.id as question_id
    "#;

        let mut props = BoltMap::new();
        props.put(BoltString::from("id"), BoltType::String(BoltString::from(question.id.as_str())));
        props.put(BoltString::from("content"), BoltType::String(BoltString::from(question.content.as_str())));

        // Manually create BoltList for the vector
        let mut vector_list = BoltList::new();
        for &value in &question.vector {
            vector_list.push(BoltType::Float(BoltFloat::new(value as f64)));
        }
        props.put(BoltString::from("vector"), BoltType::List(vector_list));

        props.put(BoltString::from("timestamp"), BoltType::String(BoltString::from(question.timestamp.to_rfc3339().as_str())));

        let mut result = self.graph.execute(query(query_str)
            .param("content", BoltType::String(BoltString::from(question.content.as_str())))
            .param("props", BoltType::Map(props))
            .param("interaction_id", BoltType::String(BoltString::from(interaction_id)))
        ).await?;

        let question_id = if let Some(row) = result.next().await? {
            row.get::<String>("question_id").map_err(|e| Neo4jClientError::OtherError(e.to_string()))?
        } else {
            return Err(Neo4jClientError::OtherError("Failed to create or update question node".to_string()));
        };

        Ok(question_id)
    }

    pub async fn create_response(&self, response: &Neo4jResponse, interaction_id: &str, model_id: &str) -> Result<String, Neo4jClientError> {
        let query_str = r#"
    CREATE (r:Response {
        id: $id,
        content: $content,
        vector: $vector,
        timestamp: $timestamp,
        confidence: $confidence,
        llm_specific_data: $llm_specific_data
    })
    WITH r
    MATCH (i:Interaction {id: $interaction_id})
    MATCH (m:Model {id: $model_id})
    CREATE (i)-[:HAS_RESPONSE]->(r)
    CREATE (r)-[:GENERATED_BY]->(m)
    RETURN r.id as response_id
    "#;

        let query = query(query_str)
            .param("id", response.id.clone())
            .param("content", response.content.clone())
            .param("vector", BoltType::List(response.vector.clone()))
            .param("timestamp", response.timestamp.to_rfc3339())
            .param("confidence", response.confidence)
            .param("llm_specific_data", serde_json::to_string(&response.llm_specific_data)?)
            .param("interaction_id", interaction_id)
            .param("model_id", model_id);

        let mut result = self.graph.execute(query).await?;

        if let Some(row) = result.next().await? {
            row.get::<String>("response_id")
                .map_err(|e| Neo4jClientError::OtherError(format!("Failed to get response_id: {}", e)))
        } else {
            Err(Neo4jClientError::OtherError("No result returned when creating response node".to_string()))
        }
    }

    // Helper function to convert JsonValue to BoltType
    fn json_to_bolt_type(&self, value: &serde_json::Value) -> Result<BoltType, Neo4jClientError> {
        match value {
            serde_json::Value::Null => Ok(BoltType::Null(BoltNull)),
            serde_json::Value::Bool(b) => Ok(BoltType::Boolean(BoltBoolean::new(*b))),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(BoltType::Integer(BoltInteger::new(i)))
                } else if let Some(f) = n.as_f64() {
                    Ok(BoltType::Float(BoltFloat::new(f)))
                } else {
                    Err(Neo4jClientError::OtherError("Invalid number type".to_string()))
                }
            },
            serde_json::Value::String(s) => Ok(BoltType::String(BoltString::from(s.as_str()))),
            serde_json::Value::Array(arr) => {
                let mut list = BoltList::new();
                for item in arr {
                    list.push(self.json_to_bolt_type(item)?);
                }
                Ok(BoltType::List(list))
            },
            serde_json::Value::Object(obj) => {
                let mut map = BoltMap::new();
                for (key, value) in obj {
                    map.put(BoltString::from(key.as_str()), self.json_to_bolt_type(value)?);
                }
                Ok(BoltType::Map(map))
            },
        }
    }

    pub async fn create_or_update_model(&self, model: &Neo4jModel) -> Result<String, Neo4jClientError> {
        let query_str = r#"
    MERGE (m:Model {name: $name})
    ON CREATE SET
        m.id = $id,
        m.version = $version
    ON MATCH SET
        m.version = $version
    RETURN m.id as model_id
    "#;

        let query = query(query_str)
            .param("id", model.id.to_string())
            .param("name", model.name.to_string())
            .param("version", model.version.to_string());

        debug!("Executing query for create_or_update_model");
        debug!("Query: {}", query_str);
        debug!("Parameters: id={}, name={}, version={}",
                 model.id, model.name, model.version);

        let mut result = self.graph.execute(query).await?;

        if let Some(row) = result.next().await? {
            let model_id: String = row.get("model_id")?;
            Ok(model_id)
        } else {
            Err(Neo4jClientError::Other("No result returned when creating or updating model node".to_string()))
        }
    }

    pub async fn create_or_get_keyword(&self, keyword: &str) -> Result<String, Neo4jClientError> {
        let query_str = r#"
    MERGE (k:Keyword {value: $value})
    ON CREATE SET k.id = $id
    RETURN k.id as keyword_id
    "#;

        let id = Uuid::new_v4().to_string();
        let query = query(query_str)
            .param("value", keyword)
            .param("id", id.clone());

        debug!("Executing query for create_or_get_keyword");
        debug!("Query: {}", query_str);
        debug!("Parameters: value={}, id={}", keyword, id);

        let mut result = self.graph.execute(query).await?;

        if let Some(row) = result.next().await? {
            match row.get::<String>("keyword_id") {
                Ok(keyword_id) => {
                    debug!("Keyword created/retrieved successfully with id: {}", keyword_id);
                    Ok(keyword_id)
                },
                Err(e) => {
                    error!("Error getting keyword_id from row: {:?}", e);
                    Err(Neo4jClientError::OtherError(format!("Failed to get keyword_id: {}", e)))
                }
            }
        } else {
            error!("No result returned when creating/getting keyword");
            Err(Neo4jClientError::OtherError("No result returned".to_string()))
        }
    }

    pub async fn create_or_get_theme(&self, theme: &str) -> Result<String, Neo4jClientError> {
        let query_str = r#"
    MERGE (t:Theme {value: $value})
    ON CREATE SET
        t.id = $id,
        t.vector = $vector
    RETURN t.id as theme_id
    "#;

        let id = Uuid::new_v4().to_string();
        let api_key = std::env::var("FLUENT_OPENAI_API_KEY_01").expect("OPENAI_API_KEY not set");
        let theme_embedding = get_openai_embedding(theme, api_key.as_str(), "text-embedding-ada-002").await?;

        let mut vector_list = BoltList::new();
        for &value in &theme_embedding {
            vector_list.push(BoltType::Float(BoltFloat::new(value as f64)));
        }

        let query = query(query_str)
            .param("value", theme)
            .param("id", id.clone())
            .param("vector", BoltType::List(vector_list));

        debug!("Executing query for create_or_get_theme");
        debug!("Query: {}", query_str);
        debug!("Parameters: value={}, id={}", theme, id);

        let mut result = self.graph.execute(query).await?;

        if let Some(row) = result.next().await? {
            match row.get::<String>("theme_id") {
                Ok(theme_id) => {
                    debug!("Theme created/retrieved successfully with id: {}", theme_id);
                    Ok(theme_id)
                },
                Err(e) => {
                    error!("Error getting theme_id from row: {:?}", e);
                    Err(Neo4jClientError::OtherError(format!("Failed to get theme_id: {}", e)))
                }
            }
        } else {
            error!("No result returned when creating/getting theme");
            Err(Neo4jClientError::OtherError("No result returned".to_string()))
        }
    }

    pub async fn create_or_get_flow_configuration(&self, config: &Neo4jFlowConfiguration) -> Result<String, Neo4jClientError> {
        let query_str = r#"
    MERGE (f:FlowConfiguration {config_hash: $config_hash})
    ON CREATE SET
        f = $props
    RETURN f.id as config_id
    "#;

        let mut props = BoltMap::new();
        props.put(BoltString::from("id"), BoltType::String(BoltString::from(config.id.as_str())));
        props.put(BoltString::from("config_hash"), BoltType::String(BoltString::from(config.config_hash.as_str())));

        // Convert config_data to BoltType
        let config_data = self.json_to_bolt_type(&config.config_data)?;
        props.put(BoltString::from("config_data"), config_data);

        let mut result = self.graph.execute(query(query_str)
            .param("config_hash", BoltType::String(BoltString::from(config.config_hash.as_str())))
            .param("props", BoltType::Map(props))
        ).await?;

        match result.next().await {
            Ok(Some(row)) => {
                row.get::<String>("config_id")
                    .map_err(|e| Neo4jClientError::OtherError(format!("Failed to get config_id: {}", e)))
            },
            Ok(None) => Err(Neo4jClientError::OtherError("No result returned".to_string())),
            Err(e) => Err(Neo4jClientError::OtherError(format!("Error fetching result: {}", e))),
        }
    }

    pub async fn create_token_usage(&self, usage: &Neo4jTokenUsage, interaction_id: &str) -> Result<String, Neo4jClientError> {
        let query_str = r#"
    MATCH (i:Interaction {id: $interaction_id})
    CREATE (t:TokenUsage {
        id: $id,
        prompt_tokens: $prompt_tokens,
        completion_tokens: $completion_tokens,
        total_tokens: $total_tokens
    })
    CREATE (i)-[:HAS_TOKEN_USAGE]->(t)
    RETURN t.id as usage_id
    "#;

        let query = query(query_str)
            .param("id", usage.id.to_string())
            .param("interaction_id", interaction_id.to_string())
            .param("prompt_tokens", usage.prompt_tokens)
            .param("completion_tokens", usage.completion_tokens)
            .param("total_tokens", usage.total_tokens);

        debug!("Executing query for create_token_usage");
        debug!("Query: {}", query_str);
        debug!("Parameters: id={}, interaction_id={}, prompt_tokens={}, completion_tokens={}, total_tokens={}",
                 usage.id, interaction_id, usage.prompt_tokens, usage.completion_tokens, usage.total_tokens);

        let mut result = self.graph.execute(query).await?;

        if let Some(row) = result.next().await? {
            let usage_id: String = row.get("usage_id")?;
            Ok(usage_id)
        } else {
            Err(Neo4jClientError::Other("No result returned when creating token usage node".to_string()))
        }
    }

    pub async fn create_response_metrics(&self, metrics: &Neo4jResponseMetrics, response_id: &str) -> Result<String, Neo4jClientError> {
        let query_str = r#"
    MATCH (r:Response {id: $response_id})
    CREATE (m:ResponseMetrics $props)
    CREATE (r)-[:HAS_METRICS]->(m)
    RETURN m.id as metrics_id
    "#;

        let mut props = BoltMap::new();
        props.put(BoltString::from("id"), BoltType::String(BoltString::from(metrics.id.as_str())));
        props.put(BoltString::from("response_time"), BoltType::Integer(BoltInteger::new(metrics.response_time.num_milliseconds())));
        props.put(BoltString::from("token_count"), BoltType::Integer(BoltInteger::new(metrics.token_count as i64)));
        props.put(BoltString::from("confidence_score"), BoltType::Float(BoltFloat::new(metrics.confidence_score)));

        let mut result = self.graph.execute(query(query_str)
            .param("response_id", BoltType::String(BoltString::from(response_id)))
            .param("props", BoltType::Map(props))
        ).await?;

        match result.next().await {
            Ok(Some(row)) => {
                row.get::<String>("metrics_id")
                    .map_err(|e| Neo4jClientError::OtherError(format!("Failed to get metrics_id: {}", e)))
            },
            Ok(None) => Err(Neo4jClientError::OtherError("No result returned".to_string())),
            Err(e) => Err(Neo4jClientError::OtherError(format!("Error fetching result: {}", e))),
        }
    }

    pub async fn find_similar_questions(&self, vector: &[f32], limit: usize) -> Result<Vec<Neo4jQuestion>, Neo4jClientError> {
        let query_str = r#"
    CALL db.index.vector.queryNodes('question_vector_index', $limit, $vector)
    YIELD node, score
    RETURN node.id as id, node.content as content, node.vector as vector, node.timestamp as timestamp, score
    ORDER BY score DESC
    "#;

        let mut vector_list = BoltList::new();
        for &value in vector {
            vector_list.push(BoltType::Float(BoltFloat::new(value as f64)));
        }

        let mut result = self.graph.execute(query(query_str)
            .param("limit", BoltType::Integer(BoltInteger::new(limit as i64)))
            .param("vector", BoltType::List(vector_list))
        ).await?;

        let mut questions = Vec::new();

        while let Some(row) = result.next().await? {
            questions.push(Neo4jQuestion {
                id: row.get("id")
                    .map_err(|e| Neo4jClientError::OtherError(format!("Failed to get id: {}", e)))?,
                content: row.get("content")
                    .map_err(|e| Neo4jClientError::OtherError(format!("Failed to get content: {}", e)))?,
                vector: row.get::<Vec<f64>>("vector")
                    .map_err(|e| Neo4jClientError::OtherError(format!("Failed to get vector: {}", e)))?
                    .into_iter()
                    .map(|v| v as f32)
                    .collect(),
                timestamp: row.get("timestamp")
                    .map_err(|e| Neo4jClientError::OtherError(format!("Failed to get timestamp: {}", e)))?,
            });
        }

        Ok(questions)
    }

    pub async fn find_similar_responses(&self, vector: &[f32], limit: usize) -> Result<Vec<Neo4jResponse>, Neo4jClientError> {
        let query = query(
            r#"
            CALL db.index.vector.queryNodes('response_vector_index', $limit, $vector)
            YIELD node, score
            RETURN node.id as id, node.content as content, node.vector as vector, node.timestamp as timestamp,
                   node.confidence as confidence, node.llm_specific_data as llm_specific_data, score
            ORDER BY score DESC
            "#
        )
            .param("limit", limit as i64)
            .param("vector", vector.to_vec());

        let mut result = self.graph.execute(query).await?;
        let mut responses = Vec::new();

        while let Some(row) = result.next().await? {
            responses.push(Neo4jResponse {
                id: row.get("id")?,
                content: row.get("content")?,
                vector: row.get("vector")?,
                timestamp: row.get("timestamp")?,
                confidence: row.get("confidence")?,
                llm_specific_data: row.get("llm_specific_data")?,
            });
        }

        Ok(responses)
    }

    pub async fn update_similarity_relationships(&self) -> Result<(), Neo4jClientError> {
        let query_str = r#"
    MATCH (q1:Question)
    WITH q1
    CALL db.index.vector.queryNodes('question_vector_index', 5, q1.vector) YIELD node as q2, score
    WHERE q1 <> q2 AND score > 0.8
    MERGE (q1)-[r:SIMILAR_TO]-(q2)
    ON CREATE SET r.score = score
    ON MATCH SET r.score = score
    RETURN count(*) as updated
    "#;

        let mut result = self.graph.execute(query(query_str)).await?;

        if let Some(row) = result.next().await? {
            let updated: i64 = row.get("updated")
                .map_err(|e| Neo4jClientError::OtherError(format!("Failed to get updated count: {}", e)))?;
            debug!("Updated {} similarity relationships", updated);
            Ok(())
        } else {
            Err(Neo4jClientError::OtherError("No result returned from similarity update query".to_string()))
        }
    }








    pub async fn extract_and_link_keywords(&self, content: &str, node_id: &str, node_type: &str) -> Result<(), Neo4jClientError> {
        debug!("Extracting keywords from content: {}", content);

        let stop_words: HashSet<String> = get(stop_words::LANGUAGE::English).into_iter().collect();
        let en_stemmer = Stemmer::create(Algorithm::English);

        // Tokenize, clean, and filter words
        let words: Vec<(String, String)> = content.split_whitespace()
            .map(|word| word.trim_matches(|c: char| !c.is_alphanumeric()))
            .filter(|word| !word.is_empty() && word.len() > 5 && !stop_words.contains(*word))
            .map(|word| {
                let cleaned = word.to_lowercase();
                (cleaned.clone(), en_stemmer.stem(&cleaned).to_string())
            })
            .collect();

        // Calculate term frequency
        let mut term_freq: HashMap<String, (String, usize)> = HashMap::new();
        for (original, stemmed) in &words {
            term_freq.entry(stemmed.clone())
                .and_modify(|(_, count)| *count += 1)
                .or_insert((original.clone(), 1));
        }

        // Update document count and word document count
        {
            let mut doc_count = self.document_count.write().unwrap();
            *doc_count += 1;
        }
        {
            let mut word_doc_count = self.word_document_count.write().unwrap();
            for stemmed in term_freq.keys() {
                *word_doc_count.entry(stemmed.clone()).or_insert(0) += 1;
            }
        }

        // Calculate TF-IDF scores
        let doc_count = *self.document_count.read().unwrap();
        let word_doc_count = self.word_document_count.read().unwrap();
        let mut tfidf_scores: Vec<(String, f64)> = term_freq.iter()
            .map(|(stemmed, (original, freq))| {
                let tf = *freq as f64 / words.len() as f64;
                let idf = (doc_count as f64 / *word_doc_count.get(stemmed).unwrap_or(&1) as f64).ln();
                (original.clone(), tf * idf)
            })
            .collect();

        // Sort by TF-IDF score and take top 3 keywords
        tfidf_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let keywords: Vec<String> = tfidf_scores.into_iter().take(3).map(|(word, _)| word).collect();

        for keyword in keywords {
            debug!("Processing keyword: {}", keyword);
            match self.create_or_get_keyword(&keyword).await {
                Ok(keyword_id) => {
                    let query_str = r#"
                MATCH (n) WHERE n.id = $node_id
                MATCH (k:Keyword) WHERE k.id = $keyword_id
                MERGE (n)-[:HAS_KEYWORD]->(k)
                RETURN count(*) as linked
                "#;

                    let query = query(query_str)
                        .param("node_id", node_id)
                        .param("keyword_id", keyword_id.clone());

                    debug!("Executing query for linking keyword");
                    debug!("Query: {}", query_str);
                    debug!("Parameters: node_id={}, keyword_id={}", node_id, keyword_id);

                    let mut result = self.graph.execute(query).await?;

                    if let Some(row) = result.next().await? {
                        let linked: i64 = row.get("linked")?;
                        if linked == 0 {
                            return Err(Neo4jClientError::OtherError(format!("Failed to link keyword {} to node {}", keyword, node_id)));
                        }
                    } else {
                        return Err(Neo4jClientError::OtherError(format!("No result returned when linking keyword {} to node {}", keyword, node_id)));
                    }
                },
                Err(e) => {
                    error!("Error creating or getting keyword '{}': {:?}", keyword, e);
                    return Err(e);
                }
            }
        }

        Ok(())
    }


    pub async fn extract_and_link_themes(&self, content: &str, node_id: &str, node_type: &str, content_embedding: &[f32]) -> Result<(), Neo4jClientError> {
        debug!("Extracting themes from content: {}", content);

        // Expanded list of themes
        let themes = vec![
            "Time", "Nature", "Technology", "Emotions", "Philosophy",
            "Science", "Art", "History", "Society", "Space",
            "Life", "Death", "Love", "War", "Peace",
            "Knowledge", "Mystery", "Adventure", "Family", "Work",
            "Artificial Intelligence", "Machine Learning", "Deep Learning", "Neural Networks", "Natural Language Processing",
            "Computer Vision", "Data Science", "Big Data", "Data Mining", "Data Analytics",
            "Cybersecurity", "Cryptography", "Blockchain", "Quantum Computing", "Cloud Computing",
            "Distributed Systems", "Internet of Things", "Edge Computing", "Virtual Reality", "Augmented Reality",
            "3D Printing", "Biotechnology", "Nanotechnology", "Robotics", "Automation",
            "Software Engineering", "Agile Methodologies", "DevOps", "Microservices", "Serverless Computing",
            "APIs", "RESTful Services", "GraphQL", "Containerization", "Docker",
            "Kubernetes", "CI/CD", "Version Control", "Git", "Networking",
            "Protocols", "Web Development", "Frontend Technologies", "Backend Technologies", "Full Stack Development",
            "Mobile Development", "Android Development", "iOS Development", "Cross-Platform Development", "Database Management",
            "SQL", "NoSQL", "Graph Databases", "Real-Time Data Processing", "Stream Processing",
            "High-Performance Computing", "Parallel Computing", "Distributed Computing", "Operating Systems", "Compilers",
            "Programming Languages", "Functional Programming", "Object-Oriented Programming", "Procedural Programming", "Scripting Languages",
            "Software Testing", "Unit Testing", "Integration Testing", "Test Automation", "Behavior-Driven Development",
            "Software Architecture", "Design Patterns", "Refactoring", "Code Review", "Technical Debt",
            "Scalability", "Performance Optimization", "Concurrency", "Multithreading", "Memory Management",
            "Algorithms", "Data Structures", "Complexity Analysis", "Graph Theory", "Combinatorics",
            "User Interface Design", "User Experience", "Accessibility", "Human-Computer Interaction", "Information Architecture",
            "Virtualization", "Infrastructure as Code", "Configuration Management", "Network Security", "Application Security",
            "Endpoint Security", "Incident Response", "Forensics", "Penetration Testing", "Vulnerability Management",
            "Compliance", "Regulatory Requirements", "Privacy", "Data Protection", "Digital Forensics",
            "Wireless Communication", "Mobile Networks", "5G", "Satellite Communication", "Optical Networks",
            "Signal Processing", "Image Processing", "Voice Recognition", "Speech Synthesis", "Bioinformatics",
            "Genomics", "Proteomics", "Health Informatics", "Telemedicine", "Wearable Technology",
            "Energy Efficiency", "Green Computing", "Smart Grids", "Renewable Energy Technologies", "Smart Cities",
            "Geospatial Technologies", "Remote Sensing", "Geographic Information Systems", "Environmental Monitoring", "Climate Modeling",
            "Financial Technology", "Algorithmic Trading", "Digital Payments", "Insurtech", "Regtech",
            "E-commerce", "Digital Marketing", "SEO", "Content Management Systems", "Digital Transformation",
            "IT Governance", "Enterprise Architecture", "Business Process Management", "Project Management", "Product Management",
            "Technical Documentation", "Knowledge Management", "Innovation Management", "Research and Development", "Intellectual Property",
            "Ethical Hacking", "Digital Identity", "Biometrics", "Smart Contracts", "Tokenization",
            "High Availability", "Fault Tolerance", "Disaster Recovery", "Backup and Restore", "Data Migration",
            "Time", "Nature", "Technology", "Emotions", "Philosophy",
            "Science", "Art", "History", "Society", "Space",
            "Life", "Death", "Love", "War", "Peace",
            "Knowledge", "Mystery", "Adventure", "Family", "Work",
            "Friendship", "Betrayal", "Freedom", "Justice", "Power",
            "Courage", "Fear", "Fantasy", "Reality", "Dreams",
            "Mythology", "Religion", "Culture", "Identity", "Memory",
            "Innovation", "Environment", "Politics", "Economics", "Health",
            "Education", "Travel", "Exploration", "Creativity", "Imagination",
            "Morality", "Ethics", "Tradition", "Progress", "Conflict",
            "Harmony", "Survival", "Transformation", "Destiny", "Fate",
            "Honor", "Glory", "Sacrifice", "Redemption", "Faith",
            "Wisdom", "Innocence", "Corruption", "Isolation", "Community",
            "Alienation", "Belonging", "Hope", "Despair", "Equality",
            "Inequality", "Oppression", "Rebellion", "Revenge", "Forgiveness",
            "Chaos", "Order", "Balance", "Duality", "Change",
            "Stability", "Ambition", "Humility", "Vanity", "Charity",
            "Greed", "Loyalty", "Deception", "Truth", "Lies",
            "Beauty", "Ugliness", "Youth", "Aging", "Patience",
            "Urgency", "Simplicity", "Complexity", "Silence", "Noise",
            "Light", "Darkness", "Joy", "Sorrow", "Success",
            "Failure", "Humor", "Melancholy", "Justice", "Injustice",
            "Wealth", "Poverty", "Respect", "Disrespect", "Honor",
            "Shame", "Pride", "Humiliation", "Safety", "Danger",
            "Curiosity", "Apathy", "Generosity", "Selfishness", "Empathy",
            "Apathy", "Resilience", "Fragility", "Heritage", "Innovation",
            "Communication", "Miscommunication", "Humanity", "Divinity", "Passion",
            "Indifference", "Surprise", "Predictability", "Empowerment", "Victimization",
            "Time", "Nature", "Technology", "Emotions", "Philosophy",
            "Science", "Art", "History", "Society", "Space",
            "Life", "Death", "Love", "War", "Peace",
            "Knowledge", "Mystery", "Adventure", "Family", "Work",
            "Ethics", "Existence", "Reality", "Consciousness", "Metaphysics",
            "Epistemology", "Logic", "Aesthetics", "Freedom", "Justice",
            "Truth", "Virtue", "Happiness", "Suffering", "Identity",
            "Mind", "Morality", "Wisdom", "Reason", "Meaning",
            "Value", "Belief", "Perception", "Dualism", "Idealism",
            "Materialism", "Nihilism", "Determinism", "Free Will", "Skepticism",
            "Human Nature", "The Self", "The Good Life", "The Absurd", "Phenomenology",
            "Ontology", "Deontology", "Consequentialism", "Utilitarianism", "Stoicism",
            "Existentialism", "Relativism", "Objectivism", "Pragmatism", "Humanism"
        ];


        let theme_embeddings = self.get_theme_embeddings(&themes).await?;

        let mut theme_scores: Vec<(String, f32)> = themes.iter().enumerate().map(|(i, &theme)| {
            let similarity = cosine_similarity(content_embedding, &theme_embeddings[i]);
            (theme.to_string(), similarity)
        }).collect();

        theme_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        debug!("Theme scores: {:?}", theme_scores);

        let threshold = -0.07; // Adjust based on your observed scores
        let top_themes: Vec<String> = theme_scores.iter()
            .filter(|(_, score)| *score > threshold)
            .take(2)
            .map(|(theme, _)| theme.clone())
            .collect();
        debug!("Top themes: {:?}", top_themes);

        for theme in top_themes {
            debug!("Processing theme: {}", theme);
            match self.create_or_get_theme(&theme).await {
                Ok(theme_id) => {
                    let query_str = r#"
                MATCH (n) WHERE n.id = $node_id
                MATCH (t:Theme) WHERE t.id = $theme_id
                MERGE (n)-[r:HAS_THEME]->(t)
                ON CREATE SET r.similarity = $similarity
                ON MATCH SET r.similarity = $similarity
                RETURN count(*) as linked
                "#;
                    let theme_id_clone = theme_id.clone();
                    let similarity = theme_scores.iter()
                        .find(|(t, _)| t == &theme)
                        .map(|(_, s)| *s)
                        .unwrap_or(0.0);

                    let query = query(query_str)
                        .param("node_id", node_id)
                        .param("theme_id", theme_id)
                        .param("similarity", similarity);

                    debug!("Executing query for linking theme");
                    debug!("Query: {}", query_str);
                    debug!("Parameters: node_id={}, theme_id={}, similarity={}", node_id, theme_id_clone, similarity);

                    let mut result = self.graph.execute(query).await?;

                    if let Some(row) = result.next().await? {
                        let linked: i64 = row.get("linked")?;
                        if linked == 0 {
                            return Err(Neo4jClientError::OtherError(format!("Failed to link theme {} to node {}", theme, node_id)));
                        }
                        debug!("Successfully linked theme {} to node {}", theme, node_id);
                    } else {
                        return Err(Neo4jClientError::OtherError(format!("No result returned when linking theme {} to node {}", theme, node_id)));
                    }
                },
                Err(e) => {
                    error!("Error creating or getting theme '{}': {:?}", theme, e);
                    return Err(e);
                }
            }
        }

        Ok(())
    }
    // Helper function to get embeddings for themes
    async fn get_theme_embeddings(&self, themes: &[&str]) -> Result<Vec<Vec<f32>>, Neo4jClientError> {
        let api_key = std::env::var("FLUENT_OPENAI_API_KEY_01").expect("OPENAI_API_KEY not set");
        let model = "text-embedding-ada-002";

        // Join all themes into a single string, separated by newlines
        let combined_themes = themes.join("\n");

        // Get embeddings for all themes in a single API call
        let embeddings = get_openai_embedding(&combined_themes, &api_key, model).await?;

        // Split the embeddings back into individual theme embeddings
        let embedding_size = embeddings.len() / themes.len();
        let theme_embeddings = embeddings.chunks(embedding_size).map(|chunk| chunk.to_vec()).collect();

        Ok(theme_embeddings)
    }




    pub async fn get_interaction_chain(&self, session_id: &str) -> Result<Vec<Neo4jInteraction>, Neo4jClientError> {
        let query = query(
            r#"
            MATCH (s:Session {id: $session_id})-[:CONTAINS]->(i:Interaction)
            OPTIONAL MATCH (i)-[:HAS_QUESTION]->(q:Question)
            OPTIONAL MATCH (i)-[:HAS_RESPONSE]->(r:Response)
            RETURN i.id as interaction_id, i.timestamp as timestamp, i.order as order,
                   q.id as question_id, q.content as question_content,
                   r.id as response_id, r.content as response_content
            ORDER BY i.order
            "#
        )
            .param("session_id", session_id);

        let mut result = self.graph.execute(query).await?;
        let mut interactions = Vec::new();

        while let Some(row) = result.next().await? {
            let empty_bolt_list = BoltList::new(); // Create an empty BoltList
            interactions.push(Neo4jInteraction {
                id: row.get("interaction_id")?,
                timestamp: row.get("timestamp")?,
                order: row.get("order")?,
                session_id: session_id.to_string(),
                question: Some(Neo4jQuestion {
                    id: row.get("question_id")?,
                    content: row.get("question_content")?,
                    vector: Vec::new(), // Keep this as Vec<f32> for Neo4jQuestion
                    timestamp: Utc::now(),
                }),
                response: Some(Neo4jResponse {
                    id: row.get("response_id")?,
                    content: row.get("response_content")?,
                    vector: empty_bolt_list, // Use the empty BoltList here
                    timestamp: Utc::now(),
                    confidence: 0.0,
                    llm_specific_data: serde_json::Value::Null,
                }),
            });
        }

        Ok(interactions)
    }


    pub async fn get_model_performance_metrics(&self, model_name: &str, start_time: DateTime<Utc>, end_time: DateTime<Utc>) -> Result<Neo4jModelPerformanceMetrics, Neo4jClientError> {
        let query_str = r#"
    MATCH (m:Model {name: $model_name})<-[:GENERATED_BY]-(r:Response)<-[:HAS_RESPONSE]-(i:Interaction)
    WHERE i.timestamp >= $start_time AND i.timestamp <= $end_time
    MATCH (r)-[:HAS_METRICS]->(rm:ResponseMetrics)
    MATCH (i)-[:HAS_TOKEN_USAGE]->(tu:TokenUsage)
    RETURN
        count(r) as response_count,
        avg(rm.response_time) as avg_response_time,
        avg(rm.confidence_score) as avg_confidence_score,
        sum(tu.total_tokens) as total_tokens_used
    "#;

        let mut result = self.graph.execute(query(query_str)
            .param("model_name", model_name)
            .param("start_time", start_time.to_rfc3339())
            .param("end_time", end_time.to_rfc3339())
        ).await?;

        if let Some(row) = result.next().await? {
            Ok(Neo4jModelPerformanceMetrics {
                model_name: model_name.to_string(),
                response_count: row.get::<i64>("response_count")?,
                avg_response_time: chrono::Duration::milliseconds(row.get::<f64>("avg_response_time")? as i64),
                avg_confidence_score: row.get::<f64>("avg_confidence_score")?,
                total_tokens_used: row.get::<i64>("total_tokens_used")?,
            })
        } else {
            Err(Neo4jClientError::OtherError("No result returned".to_string()))
        }
    }

    pub async fn get_top_keywords(&self, limit: usize) -> Result<Vec<(String, i64)>, Neo4jClientError> {
        let query_str = r#"
    MATCH (k:Keyword)<-[:HAS_KEYWORD]-()
    RETURN k.value as keyword, count(*) as usage_count
    ORDER BY usage_count DESC
    LIMIT $limit
    "#;

        let mut result = self.graph.execute(query(query_str)
            .param("limit", limit as i64)
        ).await?;

        let mut keywords = Vec::new();

        while let Some(row) = result.next().await? {
            keywords.push((
                row.get::<String>("keyword")?,
                row.get::<i64>("usage_count")?
            ));
        }

        Ok(keywords)
    }
    pub async fn get_theme_distribution(&self) -> Result<HashMap<String, i64>, Neo4jClientError> {
        let query = query(
            r#"
            MATCH (t:Theme)<-[:HAS_THEME]-()
            RETURN t.value as theme, count(*) as count
            "#
        );

        let mut result = self.graph.execute(query).await?;
        let mut distribution = HashMap::new();

        while let Some(row) = result.next().await? {
            distribution.insert(row.get::<String>("theme")?, row.get("count")?);
        }

        Ok(distribution)
    }


    pub async fn create_vector_indexes(&self) -> Result<(), Neo4jClientError> {
        let queries = vec![
            "CALL db.index.vector.createNodeIndex('question_vector_index', 'Question', 'vector', 1536, 'cosine')",
            "CALL db.index.vector.createNodeIndex('response_vector_index', 'Response', 'vector', 1536, 'cosine')",
        ];

        for query_str in queries {
            self.graph.execute(query(query_str)).await?;
        }

        Ok(())
    }


    pub async fn create_sentiment_node(&self, sentiment: f32, label: &str) -> Result<String, Neo4jClientError> {
        let query_str = format!(
            "CREATE (s:{} {{score: $score, id: $id}}) RETURN s.id as id",
            label
        );

        let id = Uuid::new_v4().to_string();
        let mut result = self.graph.execute(query(&query_str)
            .param("score", sentiment)
            .param("id", id.clone())
        ).await?;

        if let Some(row) = result.next().await? {
            Ok(row.get("id")?)
        } else {
            Err(Neo4jClientError::OtherError("Failed to create sentiment node".to_string()))
        }
    }



    pub async fn link_sentiment_to_interaction(&self, interaction_id: &str, sentiment_id: &str, relationship_type: &str) -> Result<(), Neo4jClientError> {
        let query_str = format!(
            "MATCH (i:Interaction {{id: $interaction_id}}), (s {{id: $sentiment_id}})
             CREATE (i)-[:{relationship_type}]->(s)
             RETURN count(*) as count"
        );

        let mut result = self.graph.execute(query(&query_str)
            .param("interaction_id", interaction_id)
            .param("sentiment_id", sentiment_id)
        ).await?;

        // Consume the result
        if let Some(row) = result.next().await? {
            let count: i64 = row.get("count")?;
            if count == 0 {
                return Err(Neo4jClientError::OtherError("Failed to create relationship".to_string()));
            }
        } else {
            return Err(Neo4jClientError::OtherError("No result returned from query".to_string()));
        }

        Ok(())
    }



}


// Helper function to create a vector index



fn cosine_similarity(vec1: &[f32], vec2: &[f32]) -> f32 {
    let dot_product: f32 = vec1.iter().zip(vec2.iter()).map(|(&x, &y)| x * y).sum();
    let magnitude1: f32 = vec1.iter().map(|&x| x * x).sum::<f32>().sqrt();
    let magnitude2: f32 = vec2.iter().map(|&x| x * x).sum::<f32>().sqrt();

    dot_product / (magnitude1 * magnitude2)
}

fn load_word_embeddings(path: &str) -> Result<HashMap<String, Vec<f32>>, Neo4jClientError> {
    let file = File::open(path).map_err(|e| Neo4jClientError::OtherError(format!("File error: {}", e)))?;
    let reader = BufReader::new(file);
    let mut embeddings = HashMap::new();

    for line in reader.lines() {
        let line = line.map_err(|e| Neo4jClientError::OtherError(format!("Read error: {}", e)))?;
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() > 2 {
            let word = parts[0].to_string();
            let vector: Vec<f32> = parts[1..].iter()
                .map(|&s| s.parse().unwrap_or(0.0))
                .collect();
            embeddings.insert(word, vector);
        }
    }

    Ok(embeddings)
}


fn calculate_sentiment(embedding: &[f32]) -> f32 {
    // This is a simplified approach. You might want to fine-tune this based on your specific needs.
    let embedding = Array1::from_vec(embedding.to_vec());
    let positive_direction = Array1::from_vec(vec![1.0; embedding.len()]); // Simplified positive direction

    // Calculate cosine similarity
    let dot_product = embedding.dot(&positive_direction);
    let magnitude_product = (embedding.dot(&embedding).sqrt()) * (positive_direction.dot(&positive_direction).sqrt());

    dot_product / magnitude_product
}


pub async fn capture_llm_interaction(
    neo4j_client: Arc<Neo4jClient>,
    _flow: &FlowConfig,
    prompt: &str,
    response: &str,
    model: &str,
    full_response_json: &str,
    provider: LlmProvider,

) -> Result<(), Neo4jClientError> {
    let session_id = std::env::var("FLUENT_SESSION_ID_01").expect("FLUENT_SESSION_ID_01 not set");

    debug!("Session ID: {}", session_id);
    let timestamp = Utc::now();

    debug!("Timestamp: {}", timestamp);
    let api_key = std::env::var("FLUENT_OPENAI_API_KEY_01").expect("OPENAI_API_KEY not set");

    let embedding_model = "text-embedding-ada-002";

    debug!("Getting embedding for prompt...");
    let prompt_embedding = get_openai_embedding(prompt, &api_key, embedding_model).await?;
    let prompt_embedding_clone = prompt_embedding.clone();
    debug!("Getting embedding for response...");
    let response_embedding = get_openai_embedding(response, &api_key, embedding_model).await?;

    let prompt_sentiment = calculate_sentiment(&prompt_embedding);
    let response_sentiment = calculate_sentiment(&response_embedding);
    debug!("Prompt sentiment: {}", prompt_sentiment);
    debug!("Response sentiment: {}", response_sentiment);

    debug!("Creating session node...");
    let session = Neo4jSession {
        id: session_id.clone(),
        start_time: timestamp,
        end_time: timestamp,
        context: "".to_string(),
        session_id: session_id.clone(),
        user_id: "".to_string(),
    };
    let session_node_id = neo4j_client.create_or_update_session(&session).await?;
    debug!("Session node created successfully with id: {}", session_node_id);

    debug!("Creating interaction node...");
    let interaction = Neo4jInteraction {
        id: Uuid::new_v4().to_string(),
        timestamp,
        order: 0,
        session_id: session_id.clone(),
        question: None,
        response: None,
    };
    let interaction_node_id = neo4j_client.create_interaction(&interaction).await?;
    debug!("Interaction node created successfully with id: {}", interaction_node_id);


    debug!("Creating question node...");
    let question = Neo4jQuestion {
        id: Uuid::new_v4().to_string(),
        content: prompt.to_string(),
        vector: prompt_embedding,
        timestamp,
    };
    let question_node_id = neo4j_client.create_or_update_question(&question, &interaction_node_id).await?;
    debug!("Question node created successfully with id: {}", question_node_id);


    debug!("Creating model node...");
    let model_node = Neo4jModel {
        id: Uuid::new_v4().to_string(),
        name: model.to_string(),
        version: "1.0".to_string(), // You might want to pass this as a parameter or get it from somewhere
    };
    let model_node_id = neo4j_client.create_or_update_model(&model_node).await?;
    debug!("Model node created successfully with id: {}", model_node_id);


    debug!("Creating response node...");
    let mut vector_list = BoltList::new();
    for &value in &response_embedding {
        vector_list.push(BoltType::Float(BoltFloat::new(value as f64)));
    }

    let response_node = Neo4jResponse {
        id: Uuid::new_v4().to_string(),
        content: response.to_string(),
        vector: vector_list,
        timestamp: Utc::now(),
        confidence: 1.0,
        llm_specific_data: serde_json::json!({}),  // Or any appropriate default
    };
    debug!("Creating token usage node...");
    let response_json: Result<Value, serde_json::Error> = serde_json::from_str(full_response_json);
    let response_node_id = neo4j_client.create_response(&response_node, &interaction_node_id, &model_node_id).await?;


    let token_usage_node_id = match response_json {
        Ok(json_value) => {
            match extract_token_usage(&json_value, &provider) {
                Ok(token_usage) => {
                    match neo4j_client.create_token_usage(&token_usage, &interaction_node_id).await {
                        Ok(id) => {
                            debug!("Token usage node created successfully with id: {}", id);
                            Some(id)
                        },
                        Err(e) => {
                            error!("Error creating token usage node: {:?}", e);
                            None
                        }
                    }
                },
                Err(e) => {
                    warn!("Error extracting token usage: {:?}. Continuing without token usage.", e);
                    None
                }
            }
        },
        Err(e) => {
            warn!("Error parsing full response JSON: {:?}. Continuing without token usage.", e);
            None
        }
    };

    if let Some(id) = token_usage_node_id {
        debug!("Token usage node ID: {}", id);
    } else {
        debug!("No token usage information available");
    }




    debug!("Response: {:?}", response);

    debug!("Session: {:?}", session);
    debug!("Interaction: {:?}", interaction);
    debug!("Question node: {:?}", question_node_id);
    debug!("Response node: {:?}", response_node_id);
    //debug!("Token usage: {:?}", token_usage);

    debug!("Model: {:?}", model_node);


    neo4j_client.extract_and_link_keywords(prompt, &question_node_id, "Question").await?;
    debug!("extracted keywords from prompt");

    neo4j_client.extract_and_link_keywords(response, &response_node_id, "Response").await?;
    debug!("extracted keywords from response");

    neo4j_client.extract_and_link_themes(prompt, &question_node_id, "Question", &prompt_embedding_clone).await?;
    debug!("extracted themes from prompt");

    neo4j_client.extract_and_link_themes(response, &response_node_id, "Response", &response_embedding).await?;
    debug!("extracted themes from response");

    let prompt_sentiment_id = neo4j_client.create_sentiment_node(prompt_sentiment, "PROMPT_SENTIMENT").await?;
    let response_sentiment_id = neo4j_client.create_sentiment_node(response_sentiment, "RESPONSE_SENTIMENT").await?;

    neo4j_client.link_sentiment_to_interaction(&interaction_node_id, &prompt_sentiment_id, "HAS_PROMPT_SENTIMENT").await?;
    neo4j_client.link_sentiment_to_interaction(&interaction_node_id, &response_sentiment_id, "HAS_RESPONSE_SENTIMENT").await?;



    let neo4j_client_clone = Arc::clone(&neo4j_client);

    tokio::spawn(async move {
        debug!("Updating similarity relationships...");
        if let Err(e) = neo4j_client_clone.update_similarity_relationships().await {
            debug!("Error updating similarity relationships: {:?}", e);
        }
    });

    Ok(())
}


fn extract_response_metrics(full_response: &serde_json::Value, start_time: DateTime<Utc>) -> Result<Neo4jResponseMetrics, Neo4jClientError> {
    let usage = full_response.get("usage")
        .ok_or_else(|| Neo4jClientError::OtherError("No usage data found in response".to_string()))?;

    let total_tokens = usage.get("total_tokens")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| Neo4jClientError::OtherError("No total_tokens found in usage data".to_string()))? as i32;

    // Calculate response time
    let end_time = Utc::now();
    let response_time = end_time - start_time;

    // Extract confidence score if available, otherwise use a default value
    let confidence_score = full_response.get("confidence")
        .and_then(|v| v.as_f64())
        .unwrap_or(1.0);

    Ok(Neo4jResponseMetrics {
        id: Uuid::new_v4().to_string(),
        response_time,
        token_count: total_tokens,
        confidence_score,
    })
}
fn extract_token_usage(full_response: &serde_json::Value, provider: &LlmProvider) -> Result<Neo4jTokenUsage, Neo4jClientError> {
    let usage = full_response.get("usage")
        .ok_or_else(|| Neo4jClientError::OtherError("No usage data found in response".to_string()))?;

    let (prompt_tokens, completion_tokens, total_tokens) = match provider {
        LlmProvider::Anthropic => {
            let input_tokens = usage.get("input_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32;

            let output_tokens = usage.get("output_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32;

            (input_tokens, output_tokens, input_tokens + output_tokens)
        },
        LlmProvider::OpenAI => {
            let prompt_tokens = usage.get("prompt_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32;

            let completion_tokens = usage.get("completion_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32;

            let total_tokens = usage.get("total_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or_else(|| (prompt_tokens + completion_tokens) as i64) as i32;

            (prompt_tokens, completion_tokens, total_tokens)
        },
        LlmProvider::Google => {
            let prompt_tokens = usage.get("prompt_tokens")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| Neo4jClientError::OtherError("No prompt_tokens found in usage data".to_string()))? as i32;

            let completion_tokens = usage.get("completion_tokens")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| Neo4jClientError::OtherError("No completion_tokens found in usage data".to_string()))? as i32;

            let total_tokens = usage.get("total_tokens")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| Neo4jClientError::OtherError("No total_tokens found in usage data".to_string()))? as i32;

            (prompt_tokens, completion_tokens, total_tokens)
        },
        LlmProvider::Cohere => {
            let prompt_tokens = usage.get("prompt_tokens")
                .and_then(|v| v.as_i64())
                .map(|v| v as i32)
                .unwrap_or(0);

            let completion_tokens = usage.get("completion_tokens")
                .and_then(|v| v.as_i64())
                .map(|v| v as i32)
                .unwrap_or(0);

            let total_tokens = usage.get("total_tokens")
                .and_then(|v| v.as_i64())
                .map(|v| v as i32)
                .unwrap_or(0);

            (prompt_tokens, completion_tokens, total_tokens)
        },
        // Add cases for other providers as needed
    };

    Ok(Neo4jTokenUsage {
        id: Uuid::new_v4().to_string(),
        prompt_tokens,
        completion_tokens,
        total_tokens,
    })
}