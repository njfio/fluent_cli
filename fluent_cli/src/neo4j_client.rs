use neo4rs::{Graph, query, Node as Neo4jNode, Relation, BoltString, BoltType, ConfigBuilder, Query, BoltBoolean, BoltFloat, BoltInteger, BoltMap, BoltList, BoltNull, DeError};
use serde_json::{json, Value as JsonValue};

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc, Duration};
use thiserror::Error;
use log::{debug, info, warn};
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::Read;
use std::sync::Arc;
use linfa::DatasetBase;
use linfa::traits::{Fit, Predict};
use linfa_clustering::KMeans;
use ndarray::{Array1, Array2, ArrayView, ArrayView1, Axis, Ix1};
use rand::seq::SliceRandom;
use crate::config::FlowConfig;

use tokenizers::Tokenizer;
use tokenizers::models::bpe::BPE;
use tokio::task;

use rusty_machine::learning::dbscan::DBSCAN;
use rusty_machine::prelude::{Matrix, UnSupModel};
use ndarray_stats::QuantileExt;

use linfa::prelude::*;
use ndarray_rand::RandomExt;
use rand::SeedableRng;
use rand_isaac::Isaac64Rng;


const VECTOR_SIZE: usize = 768;  // Choose an appropriate size for your vectors



#[derive(Debug, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub content: String,
    pub node_type: NodeType,
    pub attributes: HashMap<String, JsonValue>,
    pub relationships: Vec<Relationship>,
    pub metadata: Metadata,
    pub vector_representation: Option<Vec<f32>>,
    pub properties: HashMap<String, JsonValue>,
    pub version_info: VersionInfo,
    pub temporal_info: Option<TemporalInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum NodeType {
    Question,
    Answer,
    Session,
    Response,
    Model,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Relationship {
    pub relation_type: RelationType,
    pub target_id: String,
    pub properties: HashMap<String, JsonValue>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Metadata {
    pub tags: Vec<String>,
    pub source: String,
    pub timestamp: DateTime<Utc>,
    pub confidence: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VersionInfo {
    pub version: u32,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TemporalInfo {
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration: Option<std::time::Duration>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum RelationType {
    Contains,
    AnsweredBy,
    GeneratedBy,
    Uses,
}

#[derive(Error, Debug)]
pub enum Neo4jClientError {
    #[error("Neo4j error: {0}")]
    Neo4jError(#[from] neo4rs::Error),
    #[error("Other error: {0}")]
    OtherError(String),
    #[error("Deserialization error: {0}")]
    DeserializationError(#[from] serde_json::Error),
    #[error("Vector representation error: {0}")]
    VectorRepresentationError(#[from] VectorRepresentationError),
    #[error("Row error: {0}")]
    RowError(String),
    #[error("Deserialization error: {0}")]
    DeError(#[from] DeError),
}


pub struct Neo4jClient {
    graph: Graph,
    tokenizer: Arc<Tokenizer>,
}


#[derive(Debug, Serialize, Deserialize)]
pub struct Neo4jResponseData {
    pub text: String,
    pub question: String,
    pub chatId: String,
    pub chatMessageId: String,
    pub sessionId: String,
    pub memoryType: String,
    pub modelType: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryResult {
    pub entity: String,
    pub properties: HashMap<String, serde_json::Value>,
}

#[derive(Error, Debug)]
pub enum VectorRepresentationError {
    #[error("Tokenizer error: {0}")]
    TokenizerError(String),
    #[error("BPE builder error: {0}")]
    BpeBuilderError(String),
    #[error("File error: {0}")]
    FileError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Other error: {0}")]
    OtherError(String),
}

impl From<tokenizers::Error> for VectorRepresentationError {
    fn from(err: tokenizers::Error) -> Self {
        VectorRepresentationError::TokenizerError(err.to_string())
    }
}




mod duration_option_serde {
    use chrono::Duration;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(duration: &Option<Duration>, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        match duration {
            Some(d) => serializer.serialize_some(&d.num_milliseconds()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
        where
            D: Deserializer<'de>,
    {
        Option::<i64>::deserialize(deserializer).map(|opt_ms| opt_ms.map(Duration::milliseconds))
    }
}

impl Neo4jClient {

    pub async fn initialize() -> Result<Self, Neo4jClientError> {
        let neo4j_uri = env::var("NEO4J_URI").expect("NEO4J_URI must be set");
        let neo4j_user = env::var("NEO4J_USER").expect("NEO4J_USER must be set");
        let neo4j_password = env::var("NEO4J_PASSWORD").expect("NEO4J_PASSWORD must be set");
        let neo4j_db = env::var("NEO4J_DB").expect("NEO4J_DB must be set");

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
        let tokenizer = Arc::new(Self::create_tokenizer(vocab_path, merges_path)?);

        Ok(Neo4jClient { graph, tokenizer })
    }

    pub async fn add_or_update_node(&self, node: &Node) -> Result<String, Neo4jClientError> {
        match node.node_type {
            NodeType::Session => self.add_or_update_session_node(node).await,
            NodeType::Question => self.add_or_update_question_node(node).await,
            NodeType::Response => self.add_or_update_response_node(node).await,
            NodeType::Model => self.add_or_update_model_node(node).await,
            _ => Err(Neo4jClientError::OtherError("Unsupported node type".to_string())),
        }
    }

    fn create_tokenizer(vocab_path: &str, merges_path: &str) -> Result<Tokenizer, VectorRepresentationError> {
        let bpe_builder = BPE::from_file(vocab_path, merges_path);
        let bpe = bpe_builder.build()
            .map_err(|e| VectorRepresentationError::BpeBuilderError(e.to_string()))?;
        Ok(Tokenizer::new(bpe))
    }


    pub async fn create_vector_representation(&self, content: &str) -> Result<Vec<f32>, VectorRepresentationError> {
        let content = content.to_string();
        let tokenizer = Arc::clone(&self.tokenizer);

        let vector = task::spawn_blocking(move || -> Result<Vec<f32>, VectorRepresentationError> {
            let encoding = tokenizer.encode(content, false)
                .map_err(|e| VectorRepresentationError::TokenizerError(e.to_string()))?;

            let tokens = encoding.get_ids();
            let mut vector = vec![0f32; VECTOR_SIZE];
            for (i, &token) in tokens.iter().enumerate().take(VECTOR_SIZE) {
                vector[i] = token as f32;
            }
            Ok(vector)
        }).await.map_err(|e| VectorRepresentationError::OtherError(e.to_string()))??;

        Ok(vector)
    }



    async fn add_or_update_session_node(&self, node: &Node) -> Result<String, Neo4jClientError> {
        let chat_id = node.properties.get("chatId").and_then(|v| v.as_str()).ok_or_else(|| Neo4jClientError::OtherError("Chat ID not found".to_string()))?;

        let query_str =
            "MERGE (n:Session {prop_chatId: $chat_id})
             ON CREATE SET n = $props
             ON MATCH SET n += $props
             RETURN n.id as node_id";

        let props = self.flatten_node(node)?;

        let mut result = self.graph.execute(query(query_str)
            .param("chat_id", BoltType::String(BoltString::from(chat_id)))
            .param("props", BoltType::Map(props))
        ).await?;

        let node_id = if let Some(row) = result.next().await? {
            row.get::<String>("node_id").map_err(|e| Neo4jClientError::OtherError(e.to_string()))?
        } else {
            return Err(Neo4jClientError::OtherError("Failed to create or update session node".to_string()));
        };

        Ok(node_id)
    }


    async fn add_or_update_question_node(&self, node: &Node) -> Result<String, Neo4jClientError> {
        let content = &node.content;

        let query_str =
            "MERGE (n:Question {content: $content})
             ON CREATE SET n = $props
             ON MATCH SET n += $props
             RETURN n.id as node_id";

        let props = self.flatten_node(node)?;

        let mut result = self.graph.execute(query(query_str)
            .param("content", BoltType::String(BoltString::from(content.as_str())))
            .param("props", BoltType::Map(props))
        ).await?;

        let node_id = if let Some(row) = result.next().await? {
            row.get::<String>("node_id").map_err(|e| Neo4jClientError::OtherError(e.to_string()))?
        } else {
            return Err(Neo4jClientError::OtherError("Failed to create or update question node".to_string()));
        };

        Ok(node_id)
    }

    async fn add_or_update_response_node(&self, node: &Node) -> Result<String, Neo4jClientError> {
        let query_str =
            "CREATE (n:Response)
             SET n = $props
             RETURN n.id as node_id";

        let props = self.flatten_node(node)?;

        let mut result = self.graph.execute(query(query_str)
            .param("props", BoltType::Map(props))
        ).await?;

        let node_id = if let Some(row) = result.next().await? {
            row.get::<String>("node_id").map_err(|e| Neo4jClientError::OtherError(e.to_string()))?
        } else {
            return Err(Neo4jClientError::OtherError("Failed to create response node".to_string()));
        };

        Ok(node_id)
    }

    async fn add_or_update_model_node(&self, node: &Node) -> Result<String, Neo4jClientError> {
        let content = &node.content;

        let query_str =
            "MERGE (m:Model {content: $content})
             ON CREATE SET m = $props
             ON MATCH SET m += $props
             RETURN m.id as node_id";

        let props = self.flatten_node(node)?;

        let mut result = self.graph.execute(query(query_str)
            .param("content", BoltType::String(BoltString::from(content.as_str())))
            .param("props", BoltType::Map(props))
        ).await?;

        let node_id = if let Some(row) = result.next().await? {
            row.get::<String>("node_id").map_err(|e| Neo4jClientError::OtherError(e.to_string()))?
        } else {
            return Err(Neo4jClientError::OtherError("Failed to create or update model node".to_string()));
        };

        Ok(node_id)
    }

    fn flatten_node(&self, node: &Node) -> Result<BoltMap, Neo4jClientError> {
        let mut map = BoltMap::new();

        // Standard properties for all nodes
        map.put(BoltString::from("id"), BoltType::String(BoltString::from(node.id.as_str())));
        map.put(BoltString::from("content"), BoltType::String(BoltString::from(node.content.as_str())));
        map.put(BoltString::from("node_type"), BoltType::String(BoltString::from(format!("{:?}", node.node_type).as_str())));
        map.put(BoltString::from("timestamp"), BoltType::String(BoltString::from(node.metadata.timestamp.to_rfc3339().as_str())));

        // Metadata
        map.put(BoltString::from("meta_confidence"), BoltType::Float(BoltFloat::new(node.metadata.confidence)));
        map.put(BoltString::from("meta_source"), BoltType::String(BoltString::from(node.metadata.source.as_str())));
        let tags_list = BoltList::new();
        map.put(BoltString::from("meta_tags"), BoltType::List(tags_list));

        // Version info
        map.put(BoltString::from("version"), BoltType::Integer(BoltInteger::new(node.version_info.version as i64)));
        map.put(BoltString::from("created_at"), BoltType::String(BoltString::from(node.version_info.created_at.to_rfc3339().as_str())));
        map.put(BoltString::from("modified_at"), BoltType::String(BoltString::from(node.version_info.modified_at.to_rfc3339().as_str())));

        // Flatten attributes
        for (key, value) in &node.attributes {
            map.put(BoltString::from(format!("attr_{}", key).as_str()), self.json_to_bolt_type(value)?);
        }

        // Flatten properties
        for (key, value) in &node.properties {
            map.put(BoltString::from(format!("prop_{}", key).as_str()), self.json_to_bolt_type(value)?);
        }

        if let Some(vector) = &node.vector_representation {
            let mut vector_list = BoltList::new();
            for &f in vector {
                vector_list.push(BoltType::Float(BoltFloat::new(f as f64)));
            }
            map.put(BoltString::from("vector_representation"), BoltType::List(vector_list));
        }

        // Temporal info (if present)
        if let Some(temporal_info) = &node.temporal_info {
            if let Some(start_time) = temporal_info.start_time {
                map.put(BoltString::from("temporal_start_time"), BoltType::String(BoltString::from(start_time.to_rfc3339().as_str())));
            }
            if let Some(end_time) = temporal_info.end_time {
                map.put(BoltString::from("temporal_end_time"), BoltType::String(BoltString::from(end_time.to_rfc3339().as_str())));
            }
            if let Some(duration) = temporal_info.duration {
                map.put(BoltString::from("temporal_duration"), BoltType::Integer(BoltInteger::new(duration.as_millis() as i64)));
            }
        }

        Ok(map)
    }

    pub async fn add_node(&self, node: &Node) -> Result<String, Neo4jClientError> {
        let node_type_label = match node.node_type {
            NodeType::Response => "Response",
            NodeType::Question => "Question",
            NodeType::Session => "Session",
            NodeType::Answer => "Answer",
            _ => {
                return Err(Neo4jClientError::OtherError(format!("Unsupported node type: {:?}", node.node_type)))
            }
        };

        let query_str = format!(
            "CREATE (n:{} $props) RETURN n.id as node_id",
            node_type_label
        );

        let props = self.flatten_node(node)?;

        let mut result = self.graph.execute(query(&query_str).param("props", BoltType::Map(props))).await?;

        let node_id = if let Some(row) = result.next().await? {
            row.get::<String>("node_id").map_err(|e| Neo4jClientError::OtherError(e.to_string()))?
        } else {
            return Err(Neo4jClientError::OtherError("Failed to create node".to_string()));
        };

        for relationship in &node.relationships {
            self.create_relationship(&node_id, &relationship.target_id, &relationship.relation_type).await?;
        }

        Ok(node_id)
    }


    fn json_to_bolt_map(&self, value: &JsonValue) -> Result<BoltMap, Neo4jClientError> {
        let mut map = BoltMap::new();
        match value {
            JsonValue::Object(obj) => {
                for (k, v) in obj {
                    let bolt_key = BoltString::from(k.as_str());
                    let bolt_value = self.json_to_bolt_type(v)?;
                    map.put(bolt_key, bolt_value);
                }
            },
            _ => return Err(Neo4jClientError::OtherError("Expected object at top level".to_string())),
        }
        Ok(map)
    }

    fn json_to_bolt_type(&self, value: &JsonValue) -> Result<BoltType, Neo4jClientError> {
        match value {
            JsonValue::Null => Ok(BoltType::Null(Default::default())),
            JsonValue::Bool(b) => Ok(BoltType::Boolean(BoltBoolean::new(*b))),
            JsonValue::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(BoltType::Integer(BoltInteger::new(i)))
                } else if let Some(f) = n.as_f64() {
                    Ok(BoltType::Float(BoltFloat::new(f)))
                } else {
                    Err(Neo4jClientError::OtherError("Invalid number".to_string()))
                }
            },
            JsonValue::String(s) => Ok(BoltType::String(BoltString::from(s.as_str()))),
            JsonValue::Array(arr) => {
                let bolt_list = BoltList::new();
                Ok(BoltType::List(bolt_list))
            },
            JsonValue::Object(_) => {
                // Convert complex objects to JSON string
                Ok(BoltType::String(BoltString::from(value.to_string().as_str())))
            },
        }
    }
    async fn create_relationship(&self, start_id: &str, end_id: &str, rel_type: &RelationType) -> Result<(), Neo4jClientError> {
        let type_str = match rel_type {
            RelationType::Contains => "CONTAINS",
            RelationType::AnsweredBy => "ANSWERED_BY",
            RelationType::GeneratedBy => "GENERATED_BY",
            RelationType::Uses => "USES",
        };

        let query_str = format!(
            "MATCH (a), (b)
             WHERE a.id = $start_id AND b.id = $end_id
             MERGE (a)-[r:{}]->(b)
             ON CREATE SET r.created_at = datetime()
             RETURN type(r) as rel_type",
            type_str
        );

        let mut result = self.graph.execute(query(&query_str)
            .param("start_id", BoltType::String(BoltString::from(start_id)))
            .param("end_id", BoltType::String(BoltString::from(end_id)))
        ).await?;

        if result.next().await?.is_none() {
            return Err(Neo4jClientError::OtherError("Failed to create relationship".to_string()));
        }

        Ok(())
    }


    pub async fn get_or_create_node(&self, id: &str, node_type: NodeType, content: &str) -> Result<String, Neo4jClientError> {
        let node_type_label = match node_type {
            NodeType::Response => "Response",
            NodeType::Question => "Question",
            NodeType::Session => "Session",
            _ => return Err(Neo4jClientError::OtherError(format!("Unsupported node type: {:?}", node_type)))
        };

        let query_str = format!(
            "MERGE (n:{} {{id: $id}})
        ON CREATE SET n.content = $content, n.created_at = datetime(), n.label = $label
        RETURN n.id as node_id",
            node_type_label
        );

        let mut result = self.graph.execute(query(&query_str)
            .param("id", id)
            .param("content", content)
            .param("label", node_type_label)
        ).await?;

        if let Some(row) = result.next().await? {
            row.get::<String>("node_id").map_err(|e| Neo4jClientError::RowError(e.to_string()))
        } else {
            Err(Neo4jClientError::OtherError("Failed to get or create node".to_string()))
        }
    }




        pub async fn add_response_data(
            &self,
            response_data: &Neo4jResponseData,
        ) -> Result<(), Neo4jClientError> {
            // Ensure session exists based on chatId
            let session_query_str = "
            MERGE (s:Session {chatId: $chatId})
            SET s.id = $id, s.chatMessageId = $chatMessageId, s.memoryType = $memoryType
            RETURN s
        ";
            let session_q = query(session_query_str)
                .param("id", response_data.sessionId.as_str())
                .param("chatId", response_data.chatId.as_str())
                .param("chatMessageId", response_data.chatMessageId.as_str())
                .param("memoryType", response_data.memoryType.as_str());
            self.graph.run(session_q).await?;

            // Ensure question exists based on content
            let question_query_str = "
            MERGE (q:Question {content: $content})
            SET q.timestamp = $timestamp
            RETURN q
        ";
            let question_q = query(question_query_str)
                .param("content", response_data.question.as_str())
                .param("timestamp", Utc::now().to_rfc3339());
            self.graph.run(question_q).await?;

            // Create or find request based on content
            let request_query_str = "
            MERGE (r:Request {content: $content})
            SET r.id = $id, r.timestamp = $timestamp
            RETURN r
        ";
            let request_q = query(request_query_str)
                .param("id", Uuid::new_v4().to_string().as_str())
                .param("content", response_data.question.as_str())
                .param("timestamp", Utc::now().to_rfc3339());
            self.graph.run(request_q).await?;

            // Link request to question
            let request_question_rel_query_str = "
            MATCH (r:Request {content: $content}), (q:Question {content: $content})
            MERGE (r)-[:IS_QUESTION]->(q)
        ";
            let request_question_rel_q = query(request_question_rel_query_str)
                .param("content", response_data.question.as_str());
            self.graph.run(request_question_rel_q).await?;

            // Link request to session
            let request_session_rel_query_str = "
            MATCH (r:Request {content: $content}), (s:Session {chatId: $chatId})
            MERGE (r)-[:MADE_IN]->(s)
        ";
            let request_session_rel_q = query(request_session_rel_query_str)
                .param("content", response_data.question.as_str())
                .param("chatId", response_data.chatId.as_str());
            self.graph.run(request_session_rel_q).await?;

            // Create response
            let response_query_str = "
            CREATE (s:Response {id: $id, content: $content, timestamp: $timestamp, status: $status})
        ";
            let response_id = Uuid::new_v4().to_string();
            let response_q = query(response_query_str)
                .param("id", response_id.as_str())
                .param("content", response_data.text.as_str())
                .param("timestamp", Utc::now().to_rfc3339())
                .param("status", "success");
            self.graph.run(response_q).await?;

            // Link response to request
            let response_request_rel_query_str = "
            MATCH (r:Request {content: $content}), (s:Response {id: $response_id})
            MERGE (r)-[:GENERATED]->(s)
        ";
            let response_request_rel_q = query(response_request_rel_query_str)
                .param("content", response_data.question.as_str())
                .param("response_id", response_id.as_str());
            self.graph.run(response_request_rel_q).await?;

            // Ensure model exists
            let model_query_str = "
            MERGE (m:Model {type: $type})
            RETURN m
        ";
            let model_q = query(model_query_str).param("type", response_data.modelType.as_str());
            self.graph.run(model_q).await?;

            // Link response to model
            let response_model_rel_query_str = "
            MATCH (s:Response {id: $response_id}), (m:Model {type: $model_type})
            MERGE (s)-[:USED_MODEL]->(m)
        ";
            let response_model_rel_q = query(response_model_rel_query_str)
                .param("response_id", response_id.as_str())
                .param("model_type", response_data.modelType.as_str());
            self.graph.run(response_model_rel_q).await?;

            Ok(())
        }

        pub async fn query_content(&self) -> Result<Vec<QueryResult>, Neo4jClientError> {
            let query_str = "
            MATCH (n)
            WHERE (n.timestamp) IS NOT NULL
            RETURN DISTINCT 'node' AS entity, n AS properties
            LIMIT 25
            UNION ALL
            MATCH ()-[r]-()
            WHERE (r.timestamp) IS NOT NULL
            RETURN DISTINCT 'relationship' AS entity, r AS properties
            LIMIT 25
        ";
            let mut result = self.graph.execute(query(query_str)).await?;
            let mut query_results = Vec::new();

            while let Some(row) = result.next().await? {
                let entity: String = row.get("entity").map_err(|e| Neo4jClientError::OtherError(e.to_string()))?;
                let properties: HashMap<String, serde_json::Value> = row.get("properties").map_err(|e| Neo4jClientError::OtherError(e.to_string()))?;
                query_results.push(QueryResult {
                    entity,
                    properties,
                });
            }
            Ok(query_results)
        }

        pub async fn update_node_properties(&self, node_id: i64, properties: &HashMap<String, serde_json::Value>) -> Result<(), Neo4jClientError> {
            let properties_str = serde_json::to_string(properties)?;
            let query_str = format!(
                "MATCH (n) WHERE ID(n) = {} SET n += {}",
                node_id, properties_str
            );
            let q = query(&query_str);
            debug!("Update node properties Query: {}", query_str);

            self.graph.run(q).await?;
            debug!("Successfully updated properties for node {}", node_id);
            Ok(())
        }


    pub async fn find_similar_questions(&self, question: &str, limit: usize) -> Result<Vec<String>, Neo4jClientError> {
        let question_vector = self.create_vector_representation(question).await?;

        if question_vector.len() != VECTOR_SIZE {
            return Err(Neo4jClientError::OtherError(format!("Expected vector of size {}, got {}", VECTOR_SIZE, question_vector.len())));
        }

        let query_str = r#"
        MATCH (q:Question)
        WHERE q.vector_representation IS NOT NULL AND size(q.vector_representation) = $vector_size
        WITH q, gds.similarity.cosine(q.vector_representation, $vector) AS similarity
        ORDER BY similarity DESC
        LIMIT $limit
        RETURN q.content AS content, similarity
        "#;

        let mut vector_list = BoltList::new();
        for &value in &question_vector {
            vector_list.push(BoltType::Float(BoltFloat::new(value as f64)));
        }

        let mut result = self.graph.execute(query(query_str)
            .param("vector", BoltType::List(vector_list))
            .param("limit", BoltType::Integer(BoltInteger::new(limit as i64)))
            .param("vector_size", BoltType::Integer(BoltInteger::new(VECTOR_SIZE as i64)))
        ).await?;

        let mut similar_questions = Vec::new();
        while let Some(row) = result.next().await? {
            let content: String = row.get("content").map_err(|e| Neo4jClientError::OtherError(e.to_string()))?;
            similar_questions.push(content);
        }

        Ok(similar_questions)
    }

    pub async fn analyze_response_clusters(&self, n_clusters: usize) -> Result<Vec<Vec<String>>, Neo4jClientError> {
        let query_str = r#"
        MATCH (r:Response)
        WHERE r.vector_representation IS NOT NULL AND size(r.vector_representation) = $vector_size
        RETURN r.id AS id, r.vector_representation AS vector
        "#;

        let mut result = self.graph.execute(query(query_str)
            .param("vector_size", BoltType::Integer(BoltInteger::new(VECTOR_SIZE as i64)))
        ).await?;

        let mut vectors = Vec::new();
        let mut ids = Vec::new();
        while let Some(row) = result.next().await? {
            let id: String = row.get("id").map_err(|e| Neo4jClientError::OtherError(e.to_string()))?;
            let vector: Vec<f32> = row.get("vector").map_err(|e| Neo4jClientError::OtherError(e.to_string()))?;
            if vector.len() == VECTOR_SIZE {
                vectors.push(vector);
                ids.push(id);
            } else {
                warn!("Skipping vector for id {} due to incorrect size", id);
            }
        }

        if vectors.is_empty() {
            return Err(Neo4jClientError::OtherError("No valid vectors found for clustering".to_string()));
        }

        // Perform k-means clustering
        let clusters = self.kmeans(&vectors, n_clusters);

        // Group response IDs by cluster
        let mut clustered_responses = vec![Vec::new(); n_clusters];
        for (id, &cluster) in ids.iter().zip(clusters.iter()) {
            clustered_responses[cluster].push(id.clone());
        }

        Ok(clustered_responses)
    }


    fn kmeans(&self, data: &[Vec<f32>], k: usize) -> Vec<usize> {
        let n = data.len();
        let dim = data[0].len();
        debug!("Number of data points: {}", n);
        debug!("Number of dimensions: {}", dim);

        // Initialize centroids randomly
        let mut centroids: Vec<Vec<f32>> = (0..k)
            .map(|_| data.choose(&mut rand::thread_rng()).unwrap().clone())
            .collect();

        let mut assignments = vec![0; n];
        let mut changed = true;

        while changed {
            changed = false;

            // Assign points to nearest centroid
            for (i, point) in data.iter().enumerate() {
                let closest = (0..k)
                    .min_by_key(|&j| {
                        let dist: f32 = centroids[j].iter().zip(point.iter())
                            .map(|(&a, &b)| (a - b).powi(2))
                            .sum();
                        (dist * 1000.0) as i32 // Scale for integer comparison
                    })
                    .unwrap();

                if assignments[i] != closest {
                    assignments[i] = closest;
                    changed = true;
                }
            }

            // Update centroids
            for j in 0..k {
                let mut new_centroid = vec![0.0; dim];
                let mut count = 0;

                for (i, point) in data.iter().enumerate() {
                    if assignments[i] == j {
                        for d in 0..dim {
                            new_centroid[d] += point[d];
                        }
                        count += 1;
                    }
                }

                if count > 0 {
                    for d in 0..dim {
                        new_centroid[d] /= count as f32;
                    }
                    centroids[j] = new_centroid;
                }
            }
        }

        assignments
    }

    fn cosine_similarity(&self, a: &Array1<f32>, b: &Array1<f32>) -> f32 {
        let dot_product = a.dot(b);
        let norm_a = a.dot(a).sqrt();
        let norm_b = b.dot(b).sqrt();
        dot_product / (norm_a * norm_b)
    }

    pub async fn detect_anomalies(&self, threshold: f32) -> Result<Vec<String>, Neo4jClientError> {
        let vectors = self.fetch_all_vectors().await?;
        let mean_vector = vectors.mean_axis(Axis(0)).unwrap();

        let mut anomaly_indices = Vec::new();
        for (i, vector) in vectors.outer_iter().enumerate() {
            let distance = euclidean_distance(&vector, &mean_vector);
            if distance > threshold {
                anomaly_indices.push(i);
            }
        }

        // Fetch content for anomalous vectors
        let anomalous_content = self.fetch_content_for_indices(&anomaly_indices).await?;

        Ok(anomalous_content)
    }

    pub async fn cluster_vectors(&self, eps: f64, min_points: usize) -> Result<Vec<Vec<String>>, Neo4jClientError> {
        let vectors = self.fetch_all_vectors().await?;
        let mut dbscan = DBSCAN::new(eps, min_points);
        let matrix = Matrix::new(
            vectors.nrows(),
            vectors.ncols(),
            vectors.into_raw_vec().into_iter().map(|x| x as f64).collect::<Vec<f64>>()
        );

        // Train the model
        dbscan.train(&matrix).map_err(|e| Neo4jClientError::OtherError(format!("DBSCAN training error: {}", e)))?;

        // Predict clusters
        let clusters = dbscan.predict(&matrix).map_err(|e| Neo4jClientError::OtherError(format!("DBSCAN prediction error: {}", e)))?;

        // Group content by cluster
        let max_cluster = clusters.iter().flatten().max().map(|&x| x + 1).unwrap_or(0);
        let mut clustered_content = vec![Vec::new(); max_cluster];
        for (i, cluster) in clusters.iter().enumerate() {
            if let Some(c) = cluster {
                let content = self.fetch_content_for_index(i).await?;
                clustered_content[*c].push(content);
            }
        }

        Ok(clustered_content)
    }

    pub async fn analyze_trends(&self, time_window: chrono::Duration) -> Result<Vec<(String, f32)>, Neo4jClientError> {
        let query_str = r#"
        MATCH (n)
        WHERE n.timestamp > datetime() - duration($time_window)
        RETURN n.vector_representation AS vector, n.content AS content, n.timestamp AS timestamp
        ORDER BY timestamp
        "#;

        let mut result = self.graph.execute(query(query_str)
            .param("time_window", time_window.num_seconds())
        ).await?;

        let mut vectors = Vec::new();
        let mut contents = Vec::new();
        while let Some(row) = result.next().await? {
            let vector: Vec<f32> = row.get("vector").map_err(|e| Neo4jClientError::OtherError(e.to_string()))?;
            let content: String = row.get("content").map_err(|e| Neo4jClientError::OtherError(e.to_string()))?;
            vectors.push(vector);
            contents.push(content);
        }

        // Simple trend analysis: compare each vector to the average
        let avg_vector = Array2::from_shape_vec((vectors.len(), vectors[0].len()), vectors.concat())
            .map_err(|e| Neo4jClientError::OtherError(e.to_string()))?
            .mean_axis(Axis(0))
            .unwrap();

        let mut trends = Vec::new();
        for (content, vector) in contents.into_iter().zip(vectors.iter()) {
            let similarity = self.cosine_similarity(&Array1::from_vec(vector.clone()), &avg_vector);
            trends.push((content, similarity));
        }

        trends.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        Ok(trends)
    }


    pub async fn vector_arithmetic(&self, positive_terms: &[&str], negative_terms: &[&str]) -> Result<Vec<String>, Neo4jClientError> {
        let mut result_vector = Array1::zeros(VECTOR_SIZE);

        for term in positive_terms {
            let term_vector = self.create_vector_representation(term).await?;
            result_vector += &Array1::from_vec(term_vector);
        }

        for term in negative_terms {
            let term_vector = self.create_vector_representation(term).await?;
            result_vector -= &Array1::from_vec(term_vector);
        }

        // Find nearest neighbors to result_vector
        let nearest_neighbors = self.find_nearest_neighbors(&result_vector, 5).await?;

        Ok(nearest_neighbors)
    }

    // Helper functions

    async fn fetch_all_vectors(&self) -> Result<Array2<f32>, Neo4jClientError> {
        let query_str = r#"
        MATCH (n)
        WHERE n.vector_representation IS NOT NULL
        RETURN n.vector_representation AS vector
        "#;

        let mut result = self.graph.execute(query(query_str)).await?;
        let mut vectors = Vec::new();
        while let Some(row) = result.next().await? {
            let vector: Vec<f32> = row.get("vector").map_err(|e| Neo4jClientError::OtherError(e.to_string()))?;
            vectors.push(vector);
        }

        Array2::from_shape_vec((vectors.len(), vectors[0].len()), vectors.concat())
            .map_err(|e| Neo4jClientError::OtherError(e.to_string()))
    }


    async fn fetch_content_for_indices(&self, indices: &[usize]) -> Result<Vec<String>, Neo4jClientError> {
        let query_str = r#"
        MATCH (n)
        WHERE n.vector_representation IS NOT NULL
        WITH n, id(n) AS id
        WHERE id IN $indices
        RETURN n.content AS content
        "#;

        let mut indices_list = BoltList::new();
        for &i in indices {
            indices_list.push(BoltType::Integer(BoltInteger::new(i as i64)));
        }

        let mut result = self.graph.execute(query(query_str)
            .param("indices", BoltType::List(indices_list))
        ).await?;

        let mut contents = Vec::new();
        while let Some(row) = result.next().await? {
            let content: String = row.get("content")?;
            contents.push(content);
        }

        Ok(contents)
    }


    async fn fetch_content_for_index(&self, index: usize) -> Result<String, Neo4jClientError> {
        let contents = self.fetch_content_for_indices(&[index]).await?;
        contents.into_iter().next().ok_or_else(|| Neo4jClientError::OtherError("No content found for index".to_string()))
    }


    async fn find_nearest_neighbors(&self, vector: &Array1<f32>, k: usize) -> Result<Vec<String>, Neo4jClientError> {
        let query_str = r#"
        MATCH (n)
        WHERE n.vector_representation IS NOT NULL
        WITH n, gds.similarity.cosine(n.vector_representation, $vector) AS similarity
        ORDER BY similarity DESC
        LIMIT $k
        RETURN n.content AS content
        "#;

        let mut result = self.graph.execute(query(query_str)
            .param("vector", vector.as_slice())
            .param("k", k as i64)
        ).await?;

        let mut neighbors = Vec::new();
        while let Some(row) = result.next().await? {
            let content: String = row.get("content")?;
            neighbors.push(content);
        }

        Ok(neighbors)
    }



}


fn euclidean_distance(a: &ArrayView<f32, Ix1>, b: &Array1<f32>) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| (x - y).powi(2)).sum::<f32>().sqrt()
}

fn safe_preview(s: &str, max_chars: usize) -> String {
    s.chars().take(max_chars).collect::<String>() + if s.chars().count() > max_chars { "..." } else { "" }
}


pub async fn create_vector_representation(content: &str) -> Result<Vec<f32>, VectorRepresentationError> {
    debug!("Creating vector representation for content:");
    let content = content.to_string(); // Clone the content

    // Check if files exist, are readable, and contain data
    let vocab_path = "/Users/n/RustroverProjects/fluent_cli/fluent_cli/vocab.json";
    let merges_path = "/Users/n/RustroverProjects/fluent_cli/fluent_cli/merges.txt";


    debug!("Checking vocab file");
    let mut vocab_content = String::new();
    File::open(vocab_path)?.read_to_string(&mut vocab_content)?;
    if vocab_content.is_empty() {
        return Err(VectorRepresentationError::FileError("Vocab file is empty".to_string()));
    }
    debug!("Vocab file content (preview): {}", safe_preview(&vocab_content, 100));

    debug!("Checking merges file");
    let mut merges_content = String::new();
    File::open(merges_path)?.read_to_string(&mut merges_content)?;
    if merges_content.is_empty() {
        return Err(VectorRepresentationError::FileError("Merges file is empty".to_string()));
    }
    debug!("Merges file content (preview): {}", safe_preview(&merges_content, 100));

    // Spawn a blocking task for CPU-intensive tokenization
    task::spawn_blocking(move || {
        debug!("Creating BPE tokenizer and encoding:");
        // Initialize BPE
        let bpe_builder = BPE::from_file(vocab_path, merges_path);
        debug!("BPE builder created");
        let bpe = match bpe_builder.build() {
            Ok(bpe) => {
                debug!("BPE built successfully");
                bpe
            },
            Err(e) => {
                debug!("Error building BPE: {}", e);
                return Err(VectorRepresentationError::BpeBuilderError(e.to_string()));
            },
        };

        debug!("Creating tokenizer");
        let tokenizer = Tokenizer::new(bpe);

        // Tokenize the content
        debug!("Tokenizing content");
        let encoding = match tokenizer.encode(content, false) {
            Ok(encoding) => {
                debug!("Content tokenized successfully");
                encoding
            },
            Err(e) => {
                debug!("Error encoding content: {}", e);
                return Err(VectorRepresentationError::TokenizerError(e.to_string()));
            },
        };

        // Get tokens
        let tokens = encoding.get_tokens();
        debug!("Tokens (preview): {:?}", safe_preview(&format!("{:?}", tokens), 100));

        // Create vector representation
        let vector: Vec<f32> = tokens.iter()
            .map(|token| tokenizer.token_to_id(token).unwrap_or(0) as f32)
            .collect();
        debug!("Vector created with length: {}", vector.len());

        Ok(vector)
    }).await.map_err(|e| VectorRepresentationError::OtherError(e.to_string()))?
}

pub async fn capture_llm_interaction(
    neo4j_client: Arc<Neo4jClient>,
    flow: &FlowConfig,
    prompt: &str,
    response: &str,
    model: &str,
) -> Result<(), Neo4jClientError> {
    let session_id = std::env::var("FLUENT_SESSION_ID_01").expect("FLUENT_SESSION_ID_01 not set");
    let chat_id = flow.session_id.clone();
    let chat_message_id = Uuid::new_v4().to_string();
    let timestamp = Utc::now();

    // Create vector representations
    let prompt_vector = neo4j_client.create_vector_representation(prompt).await?;
    let response_vector = neo4j_client.create_vector_representation(response).await?;

    if prompt_vector.len() != VECTOR_SIZE || response_vector.len() != VECTOR_SIZE {
        return Err(Neo4jClientError::OtherError(format!("Vector size mismatch. Expected {}, got {} and {}", VECTOR_SIZE, prompt_vector.len(), response_vector.len())));
    }

    // Create or update Session Node
    let session_node = Node {
        id: chat_id.clone(),
        content: "Chat Session".to_string(),
        node_type: NodeType::Session,
        attributes: HashMap::new(),
        relationships: vec![],
        metadata: Metadata {
            tags: vec!["session".to_string()],
            source: "System".to_string(),
            timestamp,
            confidence: 1.0,
        },
        vector_representation: None,
        properties: {
            let mut props = HashMap::new();
            props.insert("chatId".to_string(), JsonValue::String(chat_id.clone()));
            props.insert("sessionId".to_string(), JsonValue::String(session_id.clone()));
            props.insert("memoryType".to_string(), JsonValue::String(flow.override_config["memoryType"].as_str().unwrap_or("Buffer Window Memory").to_string()));
            props
        },
        version_info: VersionInfo {
            version: 1,
            created_at: timestamp,
            modified_at: timestamp,
        },
        temporal_info: None,
    };

    let session_node_id = neo4j_client.add_or_update_node(&session_node).await?;

    // Create or update Model Node
    let model_node = Node {
        id: model.to_string(),
        content: model.to_string(),
        node_type: NodeType::Model,
        attributes: HashMap::new(),
        relationships: vec![],
        metadata: Metadata {
            tags: vec!["model".to_string()],
            source: "System".to_string(),
            timestamp,
            confidence: 1.0,
        },
        vector_representation: None,
        properties: {
            let mut props = HashMap::new();
            props.insert("modelType".to_string(), JsonValue::String(model.to_string()));
            props
        },
        version_info: VersionInfo {
            version: 1,
            created_at: timestamp,
            modified_at: timestamp,
        },
        temporal_info: None,
    };

    let model_node_id = neo4j_client.add_or_update_node(&model_node).await?;
    let question_node = Node {
        id: Uuid::new_v4().to_string(),
        content: prompt.to_string(),
        node_type: NodeType::Question,
        attributes: HashMap::new(),
        relationships: vec![],
        metadata: Metadata {
            tags: vec!["question".to_string()],
            source: "User".to_string(),
            timestamp,
            confidence: 1.0,
        },
        vector_representation: Some(prompt_vector.clone()),
        properties: {
            let mut props = HashMap::new();
            props.insert("chatId".to_string(), JsonValue::String(chat_id.clone()));
            props.insert("chatMessageId".to_string(), JsonValue::String(chat_message_id.clone()));
            props.insert("request".to_string(), JsonValue::String(prompt.to_string()));
            props.insert("modelType".to_string(), JsonValue::String(model.to_string()));
            props.insert("vector_representation".to_string(), json!(prompt_vector));
            props
        },
        version_info: VersionInfo {
            version: 1,
            created_at: timestamp,
            modified_at: timestamp,
        },
        temporal_info: None,
    };

    let question_node_id = neo4j_client.add_or_update_node(&question_node).await?;

    // Find similar questions
    let similar_questions = neo4j_client.find_similar_questions(prompt, 5).await?;
    info!("Similar questions: {:?}", similar_questions);
    let mut similar_questions_list = BoltList::new();
    for q in &similar_questions {
        similar_questions_list.push(BoltType::String(BoltString::from(q.as_str())));
    }
    // Update question node with similar questions
    let update_query = query(
        "MATCH (q:Question {id: $id})
         SET q.similar_questions = $similar_questions
         RETURN q"
    )
        .param("id", BoltType::String(BoltString::from(question_node_id.as_str())))
        .param("similar_questions", BoltType::List(similar_questions_list));


    neo4j_client.graph.run(update_query).await?;

    // Create Response Node
    let response_node = Node {
        id: Uuid::new_v4().to_string(),
        content: response.to_string(),
        node_type: NodeType::Response,
        attributes: HashMap::new(),
        relationships: vec![],
        metadata: Metadata {
            tags: vec!["response".to_string()],
            source: "Anthropic".to_string(),
            timestamp,
            confidence: 1.0,
        },
        vector_representation: Some(response_vector.clone()),
        properties: {
            let mut props = HashMap::new();
            props.insert("chatId".to_string(), JsonValue::String(chat_id.clone()));
            props.insert("chatMessageId".to_string(), JsonValue::String(chat_message_id.clone()));
            props.insert("sessionId".to_string(), JsonValue::String(session_id.clone()));
            props.insert("memoryType".to_string(), JsonValue::String(flow.override_config["memoryType"].as_str().unwrap_or("Buffer Window Memory").to_string()));
            props.insert("modelType".to_string(), JsonValue::String(model.to_string()));
            props.insert("vector_representation".to_string(), json!(response_vector));
            props
        },
        version_info: VersionInfo {
            version: 1,
            created_at: timestamp,
            modified_at: timestamp,
        },
        temporal_info: None,
    };

    let response_node_id = neo4j_client.add_or_update_node(&response_node).await?;

    // Create relationships
    neo4j_client.create_relationship(&session_node_id, &question_node_id, &RelationType::Contains).await?;
    neo4j_client.create_relationship(&question_node_id, &response_node_id, &RelationType::AnsweredBy).await?;
    neo4j_client.create_relationship(&response_node_id, &model_node_id, &RelationType::GeneratedBy).await?;
    neo4j_client.create_relationship(&session_node_id, &model_node_id, &RelationType::Uses).await?;

    // Analyze response clusters
    let n_clusters = 3; // You can adjust this number based on your needs
    let clustered_responses = neo4j_client.analyze_response_clusters(n_clusters).await?;
    info!("Response clusters: {:?}", clustered_responses);

    // Update response nodes with cluster information
    for (cluster_id, response_ids) in clustered_responses.iter().enumerate() {
        let mut response_ids_list = BoltList::new();
        for id in response_ids {
            response_ids_list.push(BoltType::String(BoltString::from(id.as_str())));
        }

        let update_query = query(
            "MATCH (r:Response)
             WHERE r.id IN $response_ids
             SET r.cluster = $cluster_id
             RETURN r"
        )
            .param("response_ids", BoltType::List(response_ids_list))
            .param("cluster_id", BoltType::Integer(BoltInteger::new(cluster_id as i64)));

        neo4j_client.graph.run(update_query).await?;
    }

    Ok(())
}