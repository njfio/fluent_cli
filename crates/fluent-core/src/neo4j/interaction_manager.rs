//! Interaction management for Neo4j
//! 
//! This module handles the creation and management of questions, responses,
//! and interactions in the Neo4j database for LLM conversation tracking.

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use neo4rs::{query, BoltFloat, BoltList, BoltMap, BoltString, BoltType, Graph};

use log::debug;

use crate::neo4j::query_executor::QueryExecutor;
use crate::neo4j_client::{Neo4jQuestion, Neo4jResponse, Neo4jModel, Neo4jTokenUsage};

/// Session data structure for Neo4j
#[derive(Debug, Clone)]
pub struct Neo4jSession {
    pub id: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub context: String,
    pub session_id: String,
    pub user_id: String,
}

/// Interaction data structure for Neo4j
#[derive(Debug, Clone)]
pub struct Neo4jInteraction {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub order: i32,
    pub session_id: String,
    pub question: Option<Neo4jQuestion>,
    pub response: Option<Neo4jResponse>,
}

/// Interaction statistics
#[derive(Debug, Clone)]
pub struct InteractionStats {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub response_time: f64, // in seconds
    pub finish_reason: String,
}

/// Interaction manager for Neo4j operations
pub struct InteractionManager<'a> {
    graph: &'a Graph,
    query_executor: QueryExecutor<'a>,
}

impl<'a> InteractionManager<'a> {
    pub fn new(graph: &'a Graph) -> Self {
        let query_executor = QueryExecutor::new(graph);
        Self {
            graph,
            query_executor,
        }
    }

    /// Create or update a question in Neo4j
    pub async fn create_or_update_question(
        &self,
        question: &Neo4jQuestion,
        interaction_id: &str,
    ) -> Result<String> {
        debug!("Creating or updating question for interaction {}", interaction_id);

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

        let props = self.build_question_properties(question)?;

        let mut result = self
            .graph
            .execute(
                query(query_str)
                    .param("content", question.content.as_str())
                    .param("props", BoltType::Map(props))
                    .param("interaction_id", interaction_id),
            )
            .await?;

        if let Some(row) = result.next().await? {
            Ok(row.get("question_id")?)
        } else {
            Err(anyhow!("Failed to create or update question"))
        }
    }

    /// Create a response in Neo4j
    pub async fn create_response(
        &self,
        response: &Neo4jResponse,
        interaction_id: &str,
        model_id: &str,
    ) -> Result<String> {
        debug!("Creating response for interaction {} with model {}", interaction_id, model_id);

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

        let mut result = self
            .graph
            .execute(
                query(query_str)
                    .param("id", response.id.as_str())
                    .param("content", response.content.as_str())
                    .param("vector", BoltType::List(response.vector.clone()))
                    .param("timestamp", response.timestamp.to_rfc3339().as_str())
                    .param("confidence", response.confidence)
                    .param("llm_specific_data", response.llm_specific_data.to_string().as_str())
                    .param("interaction_id", interaction_id)
                    .param("model_id", model_id),
            )
            .await?;

        if let Some(row) = result.next().await? {
            Ok(row.get("response_id")?)
        } else {
            Err(anyhow!("Failed to create response"))
        }
    }

    /// Create a session in Neo4j
    pub async fn create_session(&self, session: &Neo4jSession) -> Result<String> {
        debug!("Creating session {}", session.id);

        let query_str = r#"
        CREATE (s:Session {
            id: $id,
            start_time: $start_time,
            end_time: $end_time,
            context: $context,
            session_id: $session_id,
            user_id: $user_id
        })
        RETURN s.id as session_id
        "#;

        let mut result = self
            .graph
            .execute(
                query(query_str)
                    .param("id", session.id.as_str())
                    .param("start_time", session.start_time.to_rfc3339().as_str())
                    .param("end_time", session.end_time.to_rfc3339().as_str())
                    .param("context", session.context.as_str())
                    .param("session_id", session.session_id.as_str())
                    .param("user_id", session.user_id.as_str()),
            )
            .await?;

        if let Some(row) = result.next().await? {
            Ok(row.get("session_id")?)
        } else {
            Err(anyhow!("Failed to create session"))
        }
    }

    /// Create an interaction in Neo4j
    pub async fn create_interaction(&self, interaction: &Neo4jInteraction) -> Result<String> {
        debug!("Creating interaction {}", interaction.id);

        let query_str = r#"
        CREATE (i:Interaction {
            id: $id,
            timestamp: $timestamp,
            order: $order
        })
        WITH i
        MATCH (s:Session {id: $session_id})
        CREATE (s)-[:HAS_INTERACTION]->(i)
        RETURN i.id as interaction_id
        "#;

        let mut result = self
            .graph
            .execute(
                query(query_str)
                    .param("id", interaction.id.as_str())
                    .param("timestamp", interaction.timestamp.to_rfc3339().as_str())
                    .param("order", interaction.order)
                    .param("session_id", interaction.session_id.as_str()),
            )
            .await?;

        if let Some(row) = result.next().await? {
            Ok(row.get("interaction_id")?)
        } else {
            Err(anyhow!("Failed to create interaction"))
        }
    }

    /// Create a model in Neo4j
    pub async fn create_model(&self, model: &Neo4jModel) -> Result<String> {
        debug!("Creating model {}", model.id);

        let query_str = r#"
        MERGE (m:Model {id: $id})
        ON CREATE SET
            m.name = $name,
            m.version = $version
        RETURN m.id as model_id
        "#;

        let mut result = self
            .graph
            .execute(
                query(query_str)
                    .param("id", model.id.as_str())
                    .param("name", model.name.as_str())
                    .param("version", model.version.as_str()),
            )
            .await?;

        if let Some(row) = result.next().await? {
            Ok(row.get("model_id")?)
        } else {
            Err(anyhow!("Failed to create model"))
        }
    }

    /// Record token usage for an interaction
    pub async fn record_token_usage(
        &self,
        token_usage: &Neo4jTokenUsage,
        interaction_id: &str,
    ) -> Result<String> {
        debug!("Recording token usage for interaction {}", interaction_id);

        let query_str = r#"
        CREATE (t:TokenUsage {
            id: $id,
            prompt_tokens: $prompt_tokens,
            completion_tokens: $completion_tokens,
            total_tokens: $total_tokens
        })
        WITH t
        MATCH (i:Interaction {id: $interaction_id})
        CREATE (i)-[:HAS_TOKEN_USAGE]->(t)
        RETURN t.id as token_usage_id
        "#;

        let mut result = self
            .graph
            .execute(
                query(query_str)
                    .param("id", token_usage.id.as_str())
                    .param("prompt_tokens", token_usage.prompt_tokens)
                    .param("completion_tokens", token_usage.completion_tokens)
                    .param("total_tokens", token_usage.total_tokens)
                    .param("interaction_id", interaction_id),
            )
            .await?;

        if let Some(row) = result.next().await? {
            Ok(row.get("token_usage_id")?)
        } else {
            Err(anyhow!("Failed to record token usage"))
        }
    }

    /// Get interaction statistics
    pub async fn get_interaction_statistics(&self, session_id: &str) -> Result<InteractionStats> {
        let query_str = r#"
        MATCH (s:Session {id: $session_id})-[:HAS_INTERACTION]->(i:Interaction)
        OPTIONAL MATCH (i)-[:HAS_TOKEN_USAGE]->(t:TokenUsage)
        RETURN
            sum(t.prompt_tokens) as total_prompt_tokens,
            sum(t.completion_tokens) as total_completion_tokens,
            sum(t.total_tokens) as total_tokens,
            count(i) as interaction_count
        "#;

        let rows = self
            .query_executor
            .execute_query_with_params(
                query(query_str).param("session_id", session_id)
            )
            .await?;

        if let Some(row) = rows.first() {
            Ok(InteractionStats {
                prompt_tokens: row.get::<i64>("total_prompt_tokens").unwrap_or(0) as u32,
                completion_tokens: row.get::<i64>("total_completion_tokens").unwrap_or(0) as u32,
                total_tokens: row.get::<i64>("total_tokens").unwrap_or(0) as u32,
                response_time: 0.0, // TODO: Calculate from timestamps
                finish_reason: "completed".to_string(), // TODO: Get from actual data
            })
        } else {
            Err(anyhow!("No interaction statistics found for session {}", session_id))
        }
    }

    /// Build question properties for Neo4j
    fn build_question_properties(&self, question: &Neo4jQuestion) -> Result<BoltMap> {
        let mut props = BoltMap::new();
        
        props.put(
            BoltString::from("id"),
            BoltType::String(BoltString::from(question.id.as_str())),
        );
        
        props.put(
            BoltString::from("content"),
            BoltType::String(BoltString::from(question.content.as_str())),
        );

        let mut vector_list = BoltList::new();
        for &value in &question.vector {
            vector_list.push(BoltType::Float(BoltFloat::new(value as f64)));
        }
        props.put(BoltString::from("vector"), BoltType::List(vector_list));

        props.put(
            BoltString::from("timestamp"),
            BoltType::String(BoltString::from(question.timestamp.to_rfc3339().as_str())),
        );

        Ok(props)
    }

    /// Get recent interactions for a session
    pub async fn get_recent_interactions(&self, session_id: &str, limit: i64) -> Result<Vec<Neo4jInteraction>> {
        let query_str = r#"
        MATCH (s:Session {id: $session_id})-[:HAS_INTERACTION]->(i:Interaction)
        OPTIONAL MATCH (i)-[:HAS_QUESTION]->(q:Question)
        OPTIONAL MATCH (i)-[:HAS_RESPONSE]->(r:Response)
        RETURN i, q, r
        ORDER BY i.timestamp DESC
        LIMIT $limit
        "#;

        let rows = self
            .query_executor
            .execute_query_with_params(
                query(query_str)
                    .param("session_id", session_id)
                    .param("limit", limit)
            )
            .await?;

        let mut interactions = Vec::new();
        
        for _row in rows {
            // TODO: Parse the row data into Neo4jInteraction struct
            // This is a simplified version - in practice you'd need to properly
            // deserialize the Neo4j node data
            let interaction = Neo4jInteraction {
                id: "placeholder".to_string(),
                timestamp: Utc::now(),
                order: 0,
                session_id: session_id.to_string(),
                question: None,
                response: None,
            };
            interactions.push(interaction);
        }

        Ok(interactions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interaction_stats_creation() {
        let stats = InteractionStats {
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
            response_time: 1.5,
            finish_reason: "stop".to_string(),
        };
        
        assert_eq!(stats.total_tokens, 150);
        assert_eq!(stats.response_time, 1.5);
    }

    #[test]
    fn test_neo4j_question_creation() {
        let question = Neo4jQuestion {
            id: "test-id".to_string(),
            content: "Test question".to_string(),
            vector: vec![0.1, 0.2, 0.3],
            timestamp: Utc::now(),
        };
        
        assert_eq!(question.content, "Test question");
        assert_eq!(question.vector.len(), 3);
    }
}
