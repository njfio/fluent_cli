use neo4rs::{Graph, query, Node as Neo4jNode, Relation, BoltString, BoltType, ConfigBuilder, Query, BoltBoolean, BoltFloat, BoltInteger, BoltMap, BoltList, BoltNull};
use serde_json::Value as JsonValue;

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc, Duration};
use thiserror::Error;
use log::debug;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use crate::config::FlowConfig;


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
    #[error("Other error: {0}")]
    RowError(String),
}

pub struct Neo4jClient {
    graph: Graph,
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
        Ok(Neo4jClient { graph })
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

        // Vector representation (if present)
        if let Some(vector) = &node.vector_representation {
            let vector_list = BoltList::new();
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


}

pub async fn capture_llm_interaction(
    neo4j_client: Arc<Neo4jClient>,
    flow: &FlowConfig,
    prompt: &str,
    response: &str,
    model: &str,
) -> Result<(), Neo4jClientError> {
    let session_id = env::var("FLUENT_SESSION_ID_01").expect("FLUENT_SESSION_ID_01 not set");
    let chat_id = flow.session_id.clone();
    let chat_message_id = Uuid::new_v4().to_string();
    let timestamp = Utc::now();

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

    // Create or update Question Node
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
        vector_representation: None,
        properties: {
            let mut props = HashMap::new();
            props.insert("chatId".to_string(), JsonValue::String(chat_id.clone()));
            props.insert("chatMessageId".to_string(), JsonValue::String(chat_message_id.clone()));
            props.insert("request".to_string(), JsonValue::String(prompt.to_string()));
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

    let question_node_id = neo4j_client.add_or_update_node(&question_node).await?;

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
        vector_representation: None,
        properties: {
            let mut props = HashMap::new();
            props.insert("chatId".to_string(), JsonValue::String(chat_id.clone()));
            props.insert("chatMessageId".to_string(), JsonValue::String(chat_message_id.clone()));
            props.insert("sessionId".to_string(), JsonValue::String(session_id.clone()));
            props.insert("memoryType".to_string(), JsonValue::String(flow.override_config["memoryType"].as_str().unwrap_or("Buffer Window Memory").to_string()));
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

    let response_node_id = neo4j_client.add_or_update_node(&response_node).await?;

    // Create relationships
    neo4j_client.create_relationship(&session_node_id, &question_node_id, &RelationType::Contains).await?;
    neo4j_client.create_relationship(&question_node_id, &response_node_id, &RelationType::AnsweredBy).await?;
    neo4j_client.create_relationship(&response_node_id, &model_node_id, &RelationType::GeneratedBy).await?;
    neo4j_client.create_relationship(&session_node_id, &model_node_id, &RelationType::Uses).await?;

    Ok(())
}