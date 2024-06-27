use neo4rs::{Graph, query, Node as Neo4jNode, Relation, BoltString, BoltType, ConfigBuilder, Query, BoltBoolean, BoltFloat, BoltInteger, BoltMap, BoltList, BoltNull, DeError};
use chrono::{DateTime, Utc};
use serde_json::json;
use uuid::Uuid;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::sync::{Arc, RwLock};
use log::{debug, error, info};
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
    pub vector: Vec<f32>,
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
    CREATE (r:Response $props)
    WITH r
    MATCH (i:Interaction {id: $interaction_id})
    MATCH (m:Model {id: $model_id})
    CREATE (i)-[:HAS_RESPONSE]->(r)
    CREATE (r)-[:GENERATED_BY]->(m)
    RETURN r.id as response_id
    "#;

        let mut props = BoltMap::new();
        props.put(BoltString::from("id"), BoltType::String(BoltString::from(response.id.as_str())));
        props.put(BoltString::from("content"), BoltType::String(BoltString::from(response.content.as_str())));

        // Create BoltList for the vector
        let mut vector_list = BoltList::new();
        for &value in &response.vector {
            vector_list.push(BoltType::Float(BoltFloat::new(value as f64)));
        }
        props.put(BoltString::from("vector"), BoltType::List(vector_list));

        props.put(BoltString::from("timestamp"), BoltType::String(BoltString::from(response.timestamp.to_rfc3339().as_str())));
        props.put(BoltString::from("confidence"), BoltType::Float(BoltFloat::new(response.confidence)));

        // Convert llm_specific_data to BoltType
        let llm_data = self.json_to_bolt_type(&response.llm_specific_data)?;
        props.put(BoltString::from("llm_specific_data"), llm_data);

        let mut result = self.graph.execute(query(query_str)
            .param("props", BoltType::Map(props))
            .param("interaction_id", BoltType::String(BoltString::from(interaction_id)))
            .param("model_id", BoltType::String(BoltString::from(model_id)))
        ).await?;

        let response_id = if let Some(row) = result.next().await? {
            row.get::<String>("response_id").map_err(|e| Neo4jClientError::OtherError(e.to_string()))?
        } else {
            return Err(Neo4jClientError::OtherError("Failed to create response node".to_string()));
        };

        Ok(response_id)
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
    ON CREATE SET t.id = $id
    RETURN t.id as theme_id
    "#;

        let id = Uuid::new_v4().to_string();
        let query = query(query_str)
            .param("value", theme)
            .param("id", id.clone());

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


    pub async fn extract_and_link_themes(&self, content: &str, node_id: &str, node_type: &str) -> Result<(), Neo4jClientError> {
        debug!("Extracting themes from content: {}", content);

        // Initialize stemmer
        let en_stemmer = Stemmer::create(Algorithm::English);

        // Tokenize and count words, keeping original and stemmed versions
        let words: Vec<(String, String)> = content.split_whitespace()
            .map(|s| (s.to_lowercase(), en_stemmer.stem(&s.to_lowercase()).to_string()))
            .collect();
        debug!("Tokenized words: {:?}", words);

        let mut word_count: HashMap<String, usize> = HashMap::new();
        let mut stem_to_word: HashMap<String, String> = HashMap::new();
        for (word, stem) in &words {
            *word_count.entry(stem.clone()).or_insert(0) += 1;
            stem_to_word.entry(stem.clone()).or_insert_with(|| word.clone());
        }
        debug!("Word count: {:?}", word_count);

        // Calculate TF-IDF
        let total_words = words.len() as f32;
        let mut tfidf: HashMap<String, f32> = HashMap::new();
        for (stem, count) in word_count.iter() {
            let tf = *count as f32 / total_words;
            let idf = (1.0 + (total_words / (*count as f32))).ln();
            tfidf.insert(stem.clone(), tf * idf);
        }
        debug!("TF-IDF scores: {:?}", tfidf);


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

        // Calculate theme vectors
        let mut theme_vectors: HashMap<String, HashMap<String, f32>> = HashMap::new();
        for theme in &themes {
            let theme_words: Vec<String> = theme.split_whitespace()
                .map(|s| en_stemmer.stem(&s.to_lowercase()).to_string())
                .collect();
            let theme_vector: HashMap<String, f32> = theme_words.into_iter()
                .map(|word| (word, 1.0))
                .collect();
            theme_vectors.insert(theme.to_string(), theme_vector);
        }

        // Calculate cosine similarity between content and themes
        let mut theme_scores: Vec<(String, f32)> = themes.iter().map(|theme| {
            let theme_vector = theme_vectors.get(*theme).unwrap();
            let similarity = cosine_similarity(&tfidf, theme_vector);
            (theme.to_string(), similarity)
        }).collect();

        // Sort themes by score
        theme_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        debug!("Theme scores: {:?}", theme_scores);

        // Take top 3 themes with non-zero scores
        let top_themes: Vec<String> = theme_scores.into_iter()
            .filter(|(_, score)| *score > 0.0)
            .take(1)
            .map(|(theme, _)| theme)
            .collect();
        debug!("Top themes: {:?}", top_themes);

        for theme in top_themes {
            debug!("Processing theme: {}", theme);
            match self.create_or_get_theme(&theme).await {
                Ok(theme_id) => {
                    let query_str = r#"
                MATCH (n) WHERE n.id = $node_id
                MATCH (t:Theme) WHERE t.id = $theme_id
                MERGE (n)-[:HAS_THEME]->(t)
                RETURN count(*) as linked
                "#;

                    let query = query(query_str)
                        .param("node_id", node_id)
                        .param("theme_id", theme_id.clone());

                    debug!("Executing query for linking theme");
                    debug!("Query: {}", query_str);
                    debug!("Parameters: node_id={}, theme_id={}", node_id, theme_id);

                    let mut result = self.graph.execute(query).await?;

                    if let Some(row) = result.next().await? {
                        let linked: i64 = row.get("linked")?;
                        if linked == 0 {
                            return Err(Neo4jClientError::OtherError(format!("Failed to link theme {} to node {}", theme, node_id)));
                        }
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
            interactions.push(Neo4jInteraction {
                id: row.get("interaction_id")?,
                timestamp: row.get("timestamp")?,
                order: row.get("order")?,
                session_id: session_id.to_string(),
                question: Some(Neo4jQuestion {
                    id: row.get("question_id")?,
                    content: row.get("question_content")?,
                    vector: Vec::new(), // We're not fetching the vector here for efficiency
                    timestamp: Utc::now(), // Using current time as a placeholder
                }),
                response: Some(Neo4jResponse {
                    id: row.get("response_id")?,
                    content: row.get("response_content")?,
                    vector: Vec::new(), // We're not fetching the vector here for efficiency
                    timestamp: Utc::now(), // Using current time as a placeholder
                    confidence: 0.0, // Using a placeholder value
                    llm_specific_data: serde_json::Value::Null, // Using a placeholder value
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
}


// Helper function to create a vector index
async fn create_vector_index(graph: &Graph, label: &str, property: &str) -> Result<(), Neo4jClientError> {
    let query_str = format!(r#"
    CALL db.index.vector.createNodeIndex('{0}_{1}_index', '{0}', '{1}', 768, 'cosine')
    "#, label, property);

    graph.execute(query(&query_str)).await?;
    Ok(())
}


fn cosine_similarity(vec1: &HashMap<String, f32>, vec2: &HashMap<String, f32>) -> f32 {
    let mut dot_product = 0.0;
    let mut mag1 = 0.0;
    let mut mag2 = 0.0;

    for (word, val1) in vec1 {
        mag1 += val1 * val1;
        if let Some(val2) = vec2.get(word) {
            dot_product += val1 * val2;
        }
    }

    for val2 in vec2.values() {
        mag2 += val2 * val2;
    }

    if mag1 == 0.0 || mag2 == 0.0 {
        0.0
    } else {
        dot_product / (mag1.sqrt() * mag2.sqrt())
    }
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

pub async fn capture_llm_interaction(
    neo4j_client: Arc<Neo4jClient>,
    _flow: &FlowConfig,
    prompt: &str,
    response: &str,
    model: &str,
) -> Result<(), Neo4jClientError> {
    let session_id = std::env::var("FLUENT_SESSION_ID_01").expect("FLUENT_SESSION_ID_01 not set");
    debug!("Session ID: {}", session_id);
    let timestamp = Utc::now();
    debug!("Timestamp: {}", timestamp);

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
        vector: Vec::new(),
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
    let response_node = Neo4jResponse {
        id: Uuid::new_v4().to_string(),
        content: response.to_string(),
        vector: Vec::new(), // You might want to implement vector representation
        timestamp: Utc::now(),
        confidence: 1.0, // You might want to pass this as a parameter
        llm_specific_data: serde_json::Value::Null, // You might want to pass this as a parameter
    };
    let response_node_id = neo4j_client.create_response(&response_node, &interaction_node_id, &model_node_id).await?;
    debug!("Response node created successfully with id: {}", response_node_id);

    debug!("Creating token usage node...");
    let token_usage = Neo4jTokenUsage {
        id: Uuid::new_v4().to_string(),
        prompt_tokens: 100, // You would need to get these values from the LLM response
        completion_tokens: 50,
        total_tokens: 150,
    };
    let token_usage_node_id = match neo4j_client.create_token_usage(&token_usage, &interaction_node_id).await {
        Ok(id) => {
            debug!("Token usage node created successfully with id: {}", id);
            id
        },
        Err(e) => {
            error!("Error creating token usage node: {:?}", e);
            return Err(e);
        }
    };

    debug!("Creating response metrics node...");
    let response_metrics = Neo4jResponseMetrics {
        id: Uuid::new_v4().to_string(),
        response_time: chrono::Duration::seconds(1), // You would need to measure this
        token_count: 150,
        confidence_score: 1.0, // You might want to pass this as a parameter
    };
    let response_metrics_node_id = match neo4j_client.create_response_metrics(&response_metrics, &response_node_id).await {
        Ok(id) => {
            debug!("Response metrics node created successfully with id: {}", id);
            id
        },
        Err(e) => {
            error!("Error creating response metrics node: {:?}", e);
            return Err(e);
        }
    };

    debug!("Response: {:?}", response);

    debug!("Session: {:?}", session);
    debug!("Interaction: {:?}", interaction);
    debug!("Question node: {:?}", question_node_id);
    debug!("Response node: {:?}", response_node_id);
    debug!("Token usage: {:?}", token_usage);
    debug!("Token usage node: {:?}", token_usage_node_id);
    debug!("Response metrics: {:?}", response_metrics);
    debug!("Response metrics node: {:?}", response_metrics_node_id);
    debug!("Model: {:?}", model_node);

    debug!("Response metrics: {:?}", response_metrics);
    neo4j_client.extract_and_link_keywords(prompt, &question_node_id, "Question").await?;
    debug!("extracted keywords from prompt");
    neo4j_client.extract_and_link_keywords(response, &response_node_id, "Response").await?;
    debug!("extracted keywords from response");
    neo4j_client.extract_and_link_themes(prompt, &question_node_id, "Question").await?;
    debug!("extracted themes from prompt");
    neo4j_client.extract_and_link_themes(response, &response_node_id, "Response").await?;
    debug!("extracted themes from response");

    let neo4j_client_clone = Arc::clone(&neo4j_client);

    tokio::spawn(async move {
        debug!("Updating similarity relationships...");
        if let Err(e) = neo4j_client_clone.update_similarity_relationships().await {
            debug!("Error updating similarity relationships: {:?}", e);
        }
    });

    Ok(())
}