use anyhow::{anyhow, Error, Result};
use neo4rs::{
    query, BoltFloat, BoltInteger, BoltList, BoltMap, BoltNull, BoltString, BoltType,
    ConfigBuilder, Database, Graph, Row,
};

use chrono::Duration as ChronoDuration;

use chrono::{DateTime, Utc};
use log::{debug, error, info, warn};
use pdf_extract::extract_text;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::RwLock;
use uuid::Uuid;

use rust_stemmers::{Algorithm, Stemmer};
use serde::{Deserialize, Serialize};

use crate::config::Neo4jConfig;
use crate::traits::{DocumentProcessor, DocxProcessor};
use crate::types::DocumentStatistics;
use crate::utils::chunking::chunk_document;
use crate::voyageai_client::{get_voyage_embedding, EMBEDDING_DIMENSION};
use tokio::fs::File;
use tokio::io::AsyncReadExt;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct VoyageAIConfig {
    pub api_key: String,
    pub model: String,
}

pub struct Neo4jClient {
    graph: Graph,
    document_count: RwLock<usize>,
    word_document_count: RwLock<HashMap<String, usize>>,
    voyage_ai_config: Option<VoyageAIConfig>,
    query_llm: Option<String>,
}
impl Neo4jClient {
    pub fn get_document_count(&self) -> usize {
        self.document_count.read().map(|count| *count).unwrap_or(0)
    }
    pub fn get_word_document_count_for_word(&self, word: &str) -> usize {
        self.word_document_count
            .read()
            .map(|counts| *counts.get(word).unwrap_or(&0))
            .unwrap_or(0)
    }
    pub fn get_query_llm(&self) -> Option<&String> {
        self.query_llm.as_ref()
    }
}

#[derive(Debug, Clone)]
pub struct InteractionStats {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub response_time: f64, // in seconds
    pub finish_reason: String,
}

#[derive(Debug, Clone)]
pub struct Embedding {
    pub id: String,
    pub vector: Vec<f32>,
    pub model: String,
}

#[derive(Debug)]
pub struct EnrichmentConfig {
    pub themes_keywords_interval: ChronoDuration,
    pub clustering_interval: ChronoDuration,
    pub sentiment_interval: ChronoDuration,
}

#[derive(Debug, Clone)]
pub struct EnrichmentStatus {
    pub last_themes_keywords_update: Option<DateTime<Utc>>,
    pub last_clustering_update: Option<DateTime<Utc>>,
    pub last_sentiment_update: Option<DateTime<Utc>>,
}

impl Neo4jClient {
    pub async fn new(config: &Neo4jConfig) -> Result<Self> {
        let graph_config = ConfigBuilder::default()
            .uri(&config.uri)
            .user(&config.user)
            .password(&config.password)
            .db(Database::from(config.database.as_str())) // Convert string to Database instance
            .build()?;

        let graph = Graph::connect(graph_config).await?;

        Ok(Neo4jClient {
            graph,
            document_count: Default::default(),
            word_document_count: Default::default(),
            voyage_ai_config: config.voyage_ai.clone(),
            query_llm: config.query_llm.clone(),
        })
    }

    pub async fn ensure_indexes(&self) -> Result<()> {
        let index_queries = vec![
            "CREATE INDEX IF NOT EXISTS FOR (s:Session) ON (s.id)",
            "CREATE INDEX IF NOT EXISTS FOR (q:Question) ON (q.content)",
            "CREATE INDEX IF NOT EXISTS FOR (r:Response) ON (r.content)",
            "CREATE INDEX IF NOT EXISTS FOR (m:Model) ON (m.name)",
            "CREATE INDEX IF NOT EXISTS FOR (i:Interaction) ON (i.id)",
            "CREATE INDEX IF NOT EXISTS FOR (i:Interaction) ON (i.timestamp)",
            "CREATE INDEX IF NOT EXISTS FOR (stats:InteractionStats) ON (stats.id)",
            "CREATE INDEX IF NOT EXISTS FOR (e:Embedding) ON (e.id)",
            "CREATE INDEX IF NOT EXISTS FOR (d:Document) ON (d.id)",
            "CREATE INDEX IF NOT EXISTS FOR (c:Chunk) ON (c.id)",
        ];

        for query_str in index_queries {
            debug!("Executing index creation query: {}", query_str);
            let _ = self.graph.execute(query(query_str)).await?;
        }

        // Create a vector index for embeddings
        let vector_index_query = format!(
            "CALL db.index.vector.createNodeIndex(
                'document_embedding_index',
                'Embedding',
                'vector',
                {},
                'cosine'
            )",
            EMBEDDING_DIMENSION
        );

        match self.graph.execute(query(&vector_index_query)).await {
            Ok(_) => debug!("Vector index created successfully for Document Embedding nodes"),
            Err(e) => warn!("Failed to create vector index for Document Embedding nodes: {}. This might be normal if the index already exists.", e),
        }

        // Optionally, we can also create full-text indexes for content fields if needed
        let fulltext_index_queries = vec![
            "CALL db.index.fulltext.createNodeIndex('questionContentIndex', ['Question'], ['content'])",
            "CALL db.index.fulltext.createNodeIndex('responseContentIndex', ['Response'], ['content'])",
        ];

        for query_str in fulltext_index_queries {
            debug!("Executing full-text index creation query: {}", query_str);
            match self.graph.execute(query(query_str)).await {
                Ok(_) => debug!("Full-text index created successfully"),
                Err(e) => warn!("Failed to create full-text index: {}. This might be normal if the index already exists.", e),
            }
        }

        debug!("All indexes have been created or updated.");
        Ok(())
    }

    pub async fn create_or_update_session(&self, session: &Neo4jSession) -> Result<String> {
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

        let mut result = self
            .graph
            .execute(
                query(query_str)
                    .param("id", session.id.to_string())
                    .param("start_time", session.start_time.to_rfc3339())
                    .param("end_time", session.end_time.to_rfc3339())
                    .param("context", session.context.to_string())
                    .param("session_id", session.session_id.to_string())
                    .param("user_id", session.user_id.to_string()),
            )
            .await?;

        if let Some(row) = result.next().await? {
            Ok(row.get("session_id")?)
        } else {
            Err(anyhow::anyhow!("Failed to create or update session"))
        }
    }

    pub async fn create_interaction(
        &self,
        session_id: &str,
        request: &str,
        response: &str,
        model: &str,
        stats: &InteractionStats,
    ) -> Result<String> {
        let query_str = r#"
        MERGE (s:Session {id: $session_id})
        ON CREATE SET s.created_at = $timestamp

        MERGE (q:Question {content: $request})
        ON CREATE SET q.id = $question_id, q.timestamp = $timestamp

        MERGE (r:Response {content: $response})
        ON CREATE SET r.id = $response_id, r.timestamp = $timestamp

        MERGE (m:Model {name: $model})

        MERGE (i:Interaction {
            session_id: $session_id,
            question_content: $request,
            response_content: $response,
            model: $model
        })
        ON CREATE SET
            i.id = $id,
            i.timestamp = $timestamp

        CREATE (stats:InteractionStats {
            id: $stats_id,
            prompt_tokens: $prompt_tokens,
            completion_tokens: $completion_tokens,
            total_tokens: $total_tokens,
            response_time: $response_time,
            finish_reason: $finish_reason
        })

        MERGE (s)-[:CONTAINS]->(i)
        MERGE (i)-[:HAS_QUESTION]->(q)
        MERGE (i)-[:HAS_RESPONSE]->(r)
        MERGE (r)-[:GENERATED_BY]->(m)
        CREATE (i)-[:HAS_STATS]->(stats)

        RETURN i.id as interaction_id, q.id as question_id, r.id as response_id, m.name as model_name, stats.id as stats_id
        "#;

        let interaction_id = Uuid::new_v4().to_string();
        let question_id = Uuid::new_v4().to_string();
        let response_id = Uuid::new_v4().to_string();
        let stats_id = Uuid::new_v4().to_string();
        let timestamp = Utc::now();

        let mut result = self
            .graph
            .execute(
                query(query_str)
                    .param("session_id", BoltType::String(BoltString::from(session_id)))
                    .param(
                        "id",
                        BoltType::String(BoltString::from(interaction_id.as_str())),
                    )
                    .param(
                        "question_id",
                        BoltType::String(BoltString::from(question_id.as_str())),
                    )
                    .param(
                        "response_id",
                        BoltType::String(BoltString::from(response_id.as_str())),
                    )
                    .param(
                        "stats_id",
                        BoltType::String(BoltString::from(stats_id.as_str())),
                    )
                    .param(
                        "timestamp",
                        BoltType::String(BoltString::from(timestamp.to_rfc3339().as_str())),
                    )
                    .param("request", BoltType::String(BoltString::from(request)))
                    .param("response", BoltType::String(BoltString::from(response)))
                    .param("model", BoltType::String(BoltString::from(model)))
                    .param(
                        "prompt_tokens",
                        BoltType::Integer(BoltInteger::new(stats.prompt_tokens as i64)),
                    )
                    .param(
                        "completion_tokens",
                        BoltType::Integer(BoltInteger::new(stats.completion_tokens as i64)),
                    )
                    .param(
                        "total_tokens",
                        BoltType::Integer(BoltInteger::new(stats.total_tokens as i64)),
                    )
                    .param(
                        "response_time",
                        BoltType::Float(BoltFloat::new(stats.response_time)),
                    )
                    .param(
                        "finish_reason",
                        BoltType::String(BoltString::from(stats.finish_reason.as_str())),
                    ),
            )
            .await?;

        if let Some(row) = result.next().await? {
            let interaction_id: String = row.get("interaction_id")?;
            let question_id: String = row.get("question_id")?;
            let response_id: String = row.get("response_id")?;
            let model_name: String = row.get("model_name")?;
            let stats_id: String = row.get("stats_id")?;
            debug!("Created interaction with id: {}", interaction_id);
            debug!("Question id: {}", question_id);
            debug!("Response id: {}", response_id);
            debug!("Model name: {}", model_name);
            debug!("Stats id: {}", stats_id);

            if let Some(voyage_config) = &self.voyage_ai_config {
                debug!("Voyage AI config found, creating embeddings");
                match self
                    .create_embeddings(request, response, &question_id, &response_id, voyage_config)
                    .await
                {
                    Ok(_) => debug!("Created embeddings for interaction {}", interaction_id),
                    Err(e) => warn!(
                        "Failed to create embeddings for interaction {}: {:?}",
                        interaction_id, e
                    ),
                }

                // Call enrich_document_incrementally for both question and response
                let enrichment_config = EnrichmentConfig {
                    themes_keywords_interval: ChronoDuration::hours(1),
                    clustering_interval: ChronoDuration::days(1),
                    sentiment_interval: ChronoDuration::hours(1),
                };

                match self
                    .enrich_document_incrementally(&question_id, "Question", &enrichment_config)
                    .await
                {
                    Ok(_) => debug!("Enriched question {}", question_id),
                    Err(e) => warn!("Failed to enrich question {}: {:?}", question_id, e),
                }

                match self
                    .enrich_document_incrementally(&response_id, "Response", &enrichment_config)
                    .await
                {
                    Ok(_) => debug!("Enriched response {}", response_id),
                    Err(e) => warn!("Failed to enrich response {}: {:?}", response_id, e),
                }
            } else {
                debug!("No Voyage AI config found, skipping embedding creation and document enrichment");
            }

            Ok(interaction_id)
        } else {
            Err(anyhow::anyhow!("Failed to create interaction"))
        }
    }

    async fn create_embeddings(
        &self,
        request: &str,
        response: &str,
        question_id: &str,
        response_id: &str,
        voyage_config: &VoyageAIConfig,
    ) -> Result<()> {
        let question_embedding = get_voyage_embedding(request, voyage_config).await?;
        let response_embedding = get_voyage_embedding(response, voyage_config).await?;

        let question_embedding_node = Embedding {
            id: Uuid::new_v4().to_string(),
            vector: question_embedding,
            model: voyage_config.model.clone(),
        };

        let response_embedding_node = Embedding {
            id: Uuid::new_v4().to_string(),
            vector: response_embedding,
            model: voyage_config.model.clone(),
        };

        self.create_embedding(&question_embedding_node, question_id, "Question")
            .await?;
        self.create_embedding(&response_embedding_node, response_id, "Response")
            .await?;

        Ok(())
    }

    pub async fn create_embedding(
        &self,
        embedding: &Embedding,
        parent_id: &str,
        parent_type: &str,
    ) -> Result<String> {
        let query_str = r#"
        MATCH (parent {id: $parent_id})
        WHERE labels(parent)[0] = $parent_type
        MERGE (e:Embedding {vector: $vector})
        ON CREATE SET
            e.id = $id,
            e.model = $model,
            e.created_at = datetime()
        ON MATCH SET
            e.model = $model,
            e.updated_at = datetime()
        MERGE (parent)-[:HAS_EMBEDDING]->(e)
        RETURN e.id as embedding_id
        "#;

        let mut vector_list = BoltList::new();
        for &value in &embedding.vector {
            vector_list.push(BoltType::Float(BoltFloat::new(value as f64)));
        }

        let mut result = self
            .graph
            .execute(
                query(query_str)
                    .param("parent_id", BoltType::String(BoltString::from(parent_id)))
                    .param(
                        "parent_type",
                        BoltType::String(BoltString::from(parent_type)),
                    )
                    .param(
                        "id",
                        BoltType::String(BoltString::from(embedding.id.as_str())),
                    )
                    .param("vector", BoltType::List(vector_list))
                    .param(
                        "model",
                        BoltType::String(BoltString::from(embedding.model.as_str())),
                    ),
            )
            .await?;

        if let Some(row) = result.next().await? {
            Ok(row.get("embedding_id")?)
        } else {
            Err(anyhow::anyhow!("Failed to create or update embedding"))
        }
    }

    pub async fn upsert_document(&self, file_path: &Path, metadata: &[String]) -> Result<String> {
        debug!("Upserting document from file: {:?}", file_path);

        let content = self.extract_content(file_path).await?;

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
        .param("content", content.clone()) // Clone here
        .param("metadata", metadata)
        .param("new_metadata", metadata);

        let mut result = self.graph.execute(query).await?;

        let document_id = if let Some(row) = result.next().await? {
            row.get::<String>("document_id")?
        } else {
            return Err(anyhow!("Failed to upsert document"));
        };

        let config = EnrichmentConfig {
            themes_keywords_interval: ChronoDuration::hours(1),
            clustering_interval: ChronoDuration::days(1),
            sentiment_interval: ChronoDuration::hours(1),
        };

        let chunks = chunk_document(&content); // Now we can use content here
        self.create_chunks_and_embeddings(&document_id, &chunks)
            .await?;
        self.enrich_document_incrementally(&document_id, "Document", &config)
            .await?;
        Ok(document_id)
    }

    async fn extract_content(&self, file_path: &Path) -> Result<String> {
        let extension = file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| anyhow!("Unable to determine file type"))?;

        match extension.to_lowercase().as_str() {
            "pdf" => {
                let path_buf = file_path.to_path_buf();
                Ok(tokio::task::spawn_blocking(move || extract_text(&path_buf)).await??)
            }
            "txt" | "json" | "csv" | "tsv" | "md" | "html" | "xml" | "yml" | "yaml" | "json5"
            | "py" | "rb" | "rs" | "js" | "ts" | "php" | "java" | "c" | "cpp" | "go" | "sh"
            | "bat" | "ps1" | "psm1" | "psd1" | "ps1xml" | "psc1" | "pssc" | "pss1" | "psh" => {
                let mut file = File::open(file_path).await?;
                let mut content = String::new();
                file.read_to_string(&mut content).await?;
                Ok(content)
            }
            "docx" => {
                let processor = DocxProcessor;
                let (content, _metadata) = processor.process(file_path).await?;
                Ok(content)
            }
            // Add more file types here as needed
            _ => Err(anyhow!("Unsupported file type: {}", extension)),
        }
    }
    async fn create_chunks_and_embeddings(
        &self,
        document_id: &str,
        chunks: &[String],
    ) -> Result<()> {
        debug!(
            "Creating chunks and embeddings for document {}",
            document_id
        );
        if let Some(voyage_config) = &self.voyage_ai_config {
            for (i, chunk) in chunks.iter().enumerate() {
                let embedding = get_voyage_embedding(chunk, voyage_config).await?;

                if embedding.len() != EMBEDDING_DIMENSION {
                    return Err(anyhow!("Embedding dimension mismatch"));
                }

                let query = query(
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
                .param(
                    "document_id",
                    BoltType::String(BoltString::from(document_id)),
                )
                .param(
                    "chunk_id",
                    BoltType::String(BoltString::from(Uuid::new_v4().to_string())),
                )
                .param(
                    "content",
                    BoltType::String(BoltString::from(chunk.as_str())),
                )
                .param("index", BoltType::Integer(BoltInteger::new(i as i64)))
                .param(
                    "embedding_id",
                    BoltType::String(BoltString::from(Uuid::new_v4().to_string())),
                )
                .param("vector", embedding)
                .param(
                    "prev_chunk_id",
                    if i > 0 {
                        BoltType::String(BoltString::from(chunks[i - 1].as_str()))
                    } else {
                        BoltType::Null(BoltNull)
                    },
                );

                let mut result = self.graph.execute(query).await?;

                if result.next().await?.is_none() {
                    return Err(anyhow!("Failed to create or merge chunk and embedding"));
                }
            }
            Ok(())
        } else {
            Err(anyhow!("VoyageAI configuration not found"))
        }
    }

    pub async fn get_document_statistics(&self) -> Result<DocumentStatistics> {
        let query = query(
            "
        MATCH (d:Document)
        OPTIONAL MATCH (d)-[:HAS_CHUNK]->(c)
        OPTIONAL MATCH (c)-[:HAS_EMBEDDING]->(e)
        RETURN
            count(DISTINCT d) as document_count,
            avg(size(d.content)) as avg_content_length,
            count(DISTINCT c) as chunk_count,
            count(DISTINCT e) as embedding_count
        ",
        );

        let mut result = self.graph.execute(query).await?;

        if let Some(row) = result.next().await? {
            Ok(DocumentStatistics {
                document_count: row.get::<i64>("document_count")?,
                avg_content_length: row.get::<f64>("avg_content_length")?,
                chunk_count: row.get::<i64>("chunk_count")?,
                embedding_count: row.get::<i64>("embedding_count")?,
            })
        } else {
            Err(anyhow!("Failed to get document statistics"))
        }
    }

    pub async fn enrich_document_incrementally(
        &self,
        node_id: &str,
        node_type: &str,
        config: &EnrichmentConfig,
    ) -> Result<()> {
        debug!("Enriching {} {}", node_type, node_id);
        let status = self.get_enrichment_status(node_id, node_type).await?;
        let now = Utc::now();

        if let Some(voyage_config) = &self.voyage_ai_config {
            if status
                .last_themes_keywords_update
                .map_or(true, |last| now - last > config.themes_keywords_interval)
            {
                self.update_themes_and_keywords(node_id, node_type, voyage_config)
                    .await?;
            }

            if status
                .last_clustering_update
                .map_or(true, |last| now - last > config.clustering_interval)
            {
                self.update_clustering(node_id, node_type).await?;
            }

            if status
                .last_sentiment_update
                .map_or(true, |last| now - last > config.sentiment_interval)
            {
                self.update_sentiment(node_id, node_type).await?;
            }

            self.update_enrichment_status(node_id, node_type, &now)
                .await?;
            Ok(())
        } else {
            Err(anyhow!("VoyageAI configuration not found"))
        }
    }

    async fn get_enrichment_status(
        &self,
        node_id: &str,
        node_type: &str,
    ) -> Result<EnrichmentStatus> {
        debug!("Getting enrichment status for {} {}", node_type, node_id);
        let query = query(
            "
    MATCH (n)
    WHERE (n:Document OR n:Question OR n:Response) AND n.id = $node_id
    RETURN n.last_themes_keywords_update AS themes_keywords,
           n.last_clustering_update AS clustering,
           n.last_sentiment_update AS sentiment
    ",
        )
        .param("node_id", BoltType::String(BoltString::from(node_id)));

        let mut result = self.graph.execute(query).await?;
        if let Some(row) = result.next().await? {
            Ok(EnrichmentStatus {
                last_themes_keywords_update: row.get("themes_keywords")?,
                last_clustering_update: row.get("clustering")?,
                last_sentiment_update: row.get("sentiment")?,
            })
        } else {
            Ok(EnrichmentStatus {
                last_themes_keywords_update: None,
                last_clustering_update: None,
                last_sentiment_update: None,
            })
        }
    }

    async fn update_themes_and_keywords(
        &self,
        node_id: &str,
        node_type: &str,
        voyage_config: &VoyageAIConfig,
    ) -> Result<()> {
        debug!("Updating themes and keywords for {} {}", node_type, node_id);
        let content = self.get_node_content(node_id, node_type).await?;
        let (themes, keywords) = self
            .extract_themes_and_keywords(&content, voyage_config)
            .await?;
        self.create_theme_and_keyword_nodes(node_id, node_type, &themes, &keywords)
            .await?;
        Ok(())
    }

    async fn extract_sentiment(&self, content: &str) -> Result<f32> {
        // Define a simple sentiment lexicon
        let lexicon: HashMap<&str, f32> = [
            ("good", 1.0),
            ("great", 1.5),
            ("excellent", 2.0),
            ("amazing", 2.0),
            ("wonderful", 1.5),
            ("bad", -1.0),
            ("terrible", -1.5),
            ("awful", -2.0),
            ("horrible", -2.0),
            ("poor", -1.0),
            ("like", 0.5),
            ("love", 1.0),
            ("hate", -1.0),
            ("dislike", -0.5),
            ("happy", 1.0),
            ("sad", -1.0),
            ("angry", -1.0),
            ("joyful", 1.5),
            ("interesting", 0.5),
            ("boring", -0.5),
            ("exciting", 1.0),
            ("dull", -0.5),
        ]
        .iter()
        .cloned()
        .collect();

        let words: Vec<String> = content
            .to_lowercase()
            .split_whitespace()
            .map(String::from)
            .collect();
        let total_words = words.len() as f32;

        let sentiment_sum: f32 = words
            .iter()
            .filter_map(|word| lexicon.get(word.as_str()))
            .sum();

        // Normalize the sentiment score
        let sentiment = sentiment_sum / total_words;

        // Clamp the sentiment between -1 and 1
        Ok(sentiment.clamp(-1.0, 1.0))
    }

    async fn create_and_assign_sentiment(
        &self,
        node_id: &str,
        node_type: &str,
        sentiment: f32,
    ) -> Result<()> {
        debug!(
            "Creating and assigning sentiment node for {} {}",
            node_type, node_id
        );
        let query = query(
            "
    MATCH (n)
    WHERE (n:Document OR n:Question OR n:Response) AND n.id = $node_id
    MERGE (s:Sentiment {value: $sentiment})
    MERGE (n)-[:HAS_SENTIMENT]->(s)
    RETURN count(s) AS sentiment_count, s.value AS sentiment_value, n.id AS node_id
    ",
        )
        .param("node_id", BoltType::String(BoltString::from(node_id)))
        .param(
            "sentiment",
            BoltType::Float(BoltFloat::new(sentiment as f64)),
        );

        debug!("Executing query with sentiment: {}", sentiment);

        let result = self.graph.execute(query).await;
        match result {
            Ok(mut stream) => {
                if let Some(row) = stream.next().await? {
                    let sentiment_count: i64 = row.get("sentiment_count")?;
                    let sentiment_value: f64 = row.get("sentiment_value")?;
                    let db_node_id: String = row.get("node_id")?;
                    debug!(
                        "Created and assigned {} sentiment node with value {} for {} {}",
                        sentiment_count, sentiment_value, node_type, db_node_id
                    );
                    if sentiment_count == 0 {
                        warn!(
                            "No sentiment was created or assigned for {} {}",
                            node_type, node_id
                        );
                        return Err(anyhow!(
                            "Failed to create or assign sentiment for {} {}",
                            node_type,
                            node_id
                        ));
                    }
                } else {
                    warn!(
                        "No result returned from sentiment creation and assignment query for {} {}",
                        node_type, node_id
                    );
                    return Err(anyhow!(
                        "No result returned from sentiment creation query for {} {}",
                        node_type,
                        node_id
                    ));
                }
            }
            Err(e) => {
                error!(
                    "Error executing sentiment creation and assignment query for {} {}: {:?}",
                    node_type, node_id, e
                );
                return Err(anyhow!(
                    "Failed to create and assign sentiment node: {:?}",
                    e
                ));
            }
        }

        // Verification step
        self.verify_sentiment(node_id, sentiment).await?;

        Ok(())
    }

    async fn verify_sentiment(&self, node_id: &str, expected_sentiment: f32) -> Result<()> {
        let query = query(
            "
        MATCH (n {id: $node_id})-[:HAS_SENTIMENT]->(s:Sentiment)
        RETURN n.id as node_id, s.value as sentiment
        ",
        )
        .param("node_id", BoltType::String(BoltString::from(node_id)));

        let mut result = self.graph.execute(query).await?;
        if let Some(row) = result.next().await? {
            let db_node_id: String = row.get("node_id")?;
            let db_sentiment: f64 = row.get("sentiment")?;

            debug!("Verification for node {}", db_node_id);
            debug!("Sentiment in DB: {}", db_sentiment);

            if (db_sentiment as f32 - expected_sentiment).abs() > 1e-6 {
                warn!(
                    "Sentiment mismatch for node {}: expected {}, found {}",
                    db_node_id, expected_sentiment, db_sentiment
                );
                return Err(anyhow!("Sentiment mismatch for node {}", db_node_id));
            } else {
                debug!("Sentiment verified successfully for node {}", db_node_id);
            }
        } else {
            warn!("No sentiment found for node with ID: {}", node_id);
            return Err(anyhow!("No sentiment found for node {}", node_id));
        }
        Ok(())
    }

    async fn get_all_documents(&self) -> Result<Vec<String>> {
        let query = query(
            "
    MATCH (n)
    WHERE (n:Document OR n:Question OR n:Response)
    RETURN n.content AS content
    ",
        );

        let mut result = self.graph.execute(query).await?;
        let mut documents = Vec::new();

        while let Some(row) = result.next().await? {
            let content: String = row.get("content")?;
            documents.push(content);
        }

        Ok(documents)
    }

    async fn update_sentiment(&self, node_id: &str, node_type: &str) -> Result<()> {
        debug!("Updating sentiment for {} {}", node_type, node_id);

        // Get the content of the current node
        let content = self.get_node_content(node_id, node_type).await?;

        // Extract sentiment
        let sentiment = self.extract_sentiment(&content).await?;

        // Create and assign sentiment to the node
        match self
            .create_and_assign_sentiment(node_id, node_type, sentiment)
            .await
        {
            Ok(_) => {
                debug!(
                    "Successfully created and assigned sentiment for {} {}",
                    node_type, node_id
                );
                Ok(())
            }
            Err(e) => {
                error!(
                    "Failed to create and assign sentiment for {} {}: {:?}",
                    node_type, node_id, e
                );
                Err(e)
            }
        }
    }

    async fn update_enrichment_status(
        &self,
        node_id: &str,
        node_type: &str,
        now: &DateTime<Utc>,
    ) -> Result<()> {
        debug!("Updating enrichment status for {} {}", node_type, node_id);
        let query = query(
            "
    MATCH (n)
    WHERE (n:Document OR n:Question OR n:Response) AND n.id = $node_id
    SET n.last_themes_keywords_update = $now,
        n.last_clustering_update = $now,
        n.last_sentiment_update = $now
    ",
        )
        .param("node_id", BoltType::String(BoltString::from(node_id)))
        .param("now", BoltType::String(BoltString::from(now.to_rfc3339())));

        let _ = self.graph.execute(query).await?;
        Ok(())
    }

    async fn get_node_content(&self, node_id: &str, node_type: &str) -> Result<String> {
        debug!("Getting content for {} {}", node_type, node_type);
        let query = query(
            "
    MATCH (n)
    WHERE (n:Document OR n:Question OR n:Response) AND n.id = $node_id
    RETURN n.content AS content
    ",
        )
        .param("node_id", BoltType::String(BoltString::from(node_id)));

        let mut result = self.graph.execute(query).await?;
        if let Some(row) = result.next().await? {
            Ok(row.get("content")?)
        } else {
            Err(anyhow!("Node not found"))
        }
    }

    async fn create_theme_and_keyword_nodes(
        &self,
        node_id: &str,
        node_type: &str,
        themes: &[String],
        keywords: &[String],
    ) -> Result<()> {
        debug!(
            "Creating theme and keyword nodes for {} {}",
            node_type, node_id
        );
        let query = query(
            "
    MATCH (n)
    WHERE (n:Document OR n:Question OR n:Response) AND n.id = $node_id
    WITH n
    UNWIND $themes AS theme_name
    MERGE (t:Theme {name: theme_name})
    MERGE (n)-[:HAS_THEME]->(t)
    WITH n, collect(t) AS themes
    UNWIND $keywords AS keyword_name
    MERGE (k:Keyword {name: keyword_name})
    MERGE (n)-[:HAS_KEYWORD]->(k)
    WITH n, themes, collect(k) AS keywords
    RETURN size(themes) + size(keywords) AS total_count
    ",
        )
        .param("node_id", BoltType::String(BoltString::from(node_id)))
        .param("themes", themes)
        .param("keywords", keywords);

        debug!(
            "Executing query with themes: {:?} and keywords: {:?}",
            themes, keywords
        );

        let result = self.graph.execute(query).await;
        match result {
            Ok(mut stream) => {
                if let Some(row) = stream.next().await? {
                    let total_count: i64 = row.get("total_count")?;
                    debug!(
                        "Created {} theme and keyword nodes for {} {}",
                        total_count, node_type, node_id
                    );
                    if total_count == 0 {
                        warn!(
                            "No themes or keywords were created for {} {}",
                            node_type, node_id
                        );
                    }
                } else {
                    warn!(
                        "No result returned from theme and keyword creation query for {} {}",
                        node_type, node_id
                    );
                }
            }
            Err(e) => {
                error!(
                    "Error executing theme and keyword creation query for {} {}: {:?}",
                    node_type, node_id, e
                );
                return Err(anyhow!("Failed to create theme and keyword nodes: {:?}", e));
            }
        }

        // Verification step
        self.verify_themes_and_keywords(node_id, themes, keywords)
            .await?;

        Ok(())
    }

    async fn verify_themes_and_keywords(
        &self,
        node_id: &str,
        themes: &[String],
        keywords: &[String],
    ) -> Result<()> {
        let query = query(
            "
    MATCH (n {id: $node_id})
    OPTIONAL MATCH (n)-[:HAS_THEME]->(t:Theme)
    OPTIONAL MATCH (n)-[:HAS_KEYWORD]->(k:Keyword)
    RETURN
        n.id as node_id,
        collect(distinct t.name) as themes,
        collect(distinct k.name) as keywords,
        count(distinct t) as theme_count,
        count(distinct k) as keyword_count
    ",
        )
        .param("node_id", BoltType::String(BoltString::from(node_id)));

        let mut result = self.graph.execute(query).await?;
        if let Some(row) = result.next().await? {
            let db_node_id: String = row.get("node_id")?;
            let db_themes: Vec<String> = row.get("themes")?;
            let db_keywords: Vec<String> = row.get("keywords")?;
            let theme_count: i64 = row.get("theme_count")?;
            let keyword_count: i64 = row.get("keyword_count")?;

            debug!("Verification for node {}", db_node_id);
            debug!("Themes in DB: {:?} (count: {})", db_themes, theme_count);
            debug!(
                "Keywords in DB: {:?} (count: {})",
                db_keywords, keyword_count
            );

            let missing_themes: Vec<_> = themes
                .iter()
                .filter(|t| !db_themes.contains(t))
                .cloned()
                .collect();
            let extra_themes: Vec<_> = db_themes
                .iter()
                .filter(|t| !themes.contains(t))
                .cloned()
                .collect();
            let missing_keywords: Vec<_> = keywords
                .iter()
                .filter(|k| !db_keywords.contains(k))
                .cloned()
                .collect();
            let extra_keywords: Vec<_> = db_keywords
                .iter()
                .filter(|k| !keywords.contains(k))
                .cloned()
                .collect();

            if !missing_themes.is_empty()
                || !missing_keywords.is_empty()
                || !extra_themes.is_empty()
                || !extra_keywords.is_empty()
            {
                warn!("Discrepancies found for node {}:", db_node_id);
                if !missing_themes.is_empty() {
                    warn!("Missing themes: {:?}", missing_themes);
                }
                if !extra_themes.is_empty() {
                    warn!("Extra themes in DB: {:?}", extra_themes);
                }
                if !missing_keywords.is_empty() {
                    warn!("Missing keywords: {:?}", missing_keywords);
                }
                if !extra_keywords.is_empty() {
                    warn!("Extra keywords in DB: {:?}", extra_keywords);
                }
                return Err(anyhow!(
                    "Discrepancies found in themes or keywords for node {}",
                    db_node_id
                ));
            } else {
                debug!(
                    "All themes and keywords verified successfully for node {}",
                    db_node_id
                );
            }
        } else {
            warn!("No node found with ID: {}", node_id);
        }
        Ok(())
    }

    async fn update_clustering(&self, node_id: &str, node_type: &str) -> Result<()> {
        debug!("Updating clustering for {} {}", node_type, node_id);

        // Get the content of the current node
        let content = self.get_node_content(node_id, node_type).await?;

        // Get all documents (you may want to limit this or use a more efficient approach for large datasets)
        let all_documents = self.get_all_documents().await?;

        // Extract clusters
        let clusters = self.extract_clusters(&content, &all_documents).await?;

        // Create and assign clusters to the node
        self.create_and_assign_clusters(node_id, node_type, &clusters)
            .await?;

        Ok(())
    }

    async fn create_and_assign_clusters(
        &self,
        node_id: &str,
        node_type: &str,
        clusters: &[String],
    ) -> Result<()> {
        debug!(
            "Creating and assigning cluster nodes for {} {}",
            node_type, node_id
        );
        let query = query(
            "
        MATCH (n)
        WHERE (n:Document OR n:Question OR n:Response) AND n.id = $node_id
        WITH n
        UNWIND $clusters AS cluster_name
        MERGE (c:Cluster {name: cluster_name})
        MERGE (n)-[:BELONGS_TO]->(c)
        WITH n, collect(c) AS clusters
        RETURN size(clusters) AS total_count
        ",
        )
        .param("node_id", BoltType::String(BoltString::from(node_id)))
        .param("clusters", clusters);

        debug!("Executing query with clusters: {:?}", clusters);

        let result = self.graph.execute(query).await;
        match result {
            Ok(mut stream) => {
                if let Some(row) = stream.next().await? {
                    let total_count: i64 = row.get("total_count")?;
                    debug!(
                        "Created and assigned {} cluster nodes for {} {}",
                        total_count, node_type, node_id
                    );
                    if total_count == 0 {
                        warn!(
                            "No clusters were created or assigned for {} {}",
                            node_type, node_id
                        );
                    }
                } else {
                    warn!(
                        "No result returned from cluster creation and assignment query for {} {}",
                        node_type, node_id
                    );
                }
            }
            Err(e) => {
                error!(
                    "Error executing cluster creation and assignment query for {} {}: {:?}",
                    node_type, node_id, e
                );
                return Err(anyhow!(
                    "Failed to create and assign cluster nodes: {:?}",
                    e
                ));
            }
        }

        // Verification step
        self.verify_clusters(node_id, clusters).await?;

        Ok(())
    }

    async fn verify_clusters(&self, node_id: &str, expected_clusters: &[String]) -> Result<()> {
        let query = query(
            "
        MATCH (n {id: $node_id})
        OPTIONAL MATCH (n)-[:BELONGS_TO]->(c:Cluster)
        RETURN
            n.id as node_id,
            collect(distinct c.name) as clusters,
            count(distinct c) as cluster_count
        ",
        )
        .param("node_id", BoltType::String(BoltString::from(node_id)));

        let mut result = self.graph.execute(query).await?;
        if let Some(row) = result.next().await? {
            let db_node_id: String = row.get("node_id")?;
            let db_clusters: Vec<String> = row.get("clusters")?;
            let cluster_count: i64 = row.get("cluster_count")?;

            debug!("Verification for node {}", db_node_id);
            debug!(
                "Clusters in DB: {:?} (count: {})",
                db_clusters, cluster_count
            );

            let missing_clusters: Vec<_> = expected_clusters
                .iter()
                .filter(|c| !db_clusters.contains(c))
                .cloned()
                .collect();
            let extra_clusters: Vec<_> = db_clusters
                .iter()
                .filter(|c| !expected_clusters.contains(c))
                .cloned()
                .collect();

            if !missing_clusters.is_empty() || !extra_clusters.is_empty() {
                warn!("Discrepancies found for node {}:", db_node_id);
                if !missing_clusters.is_empty() {
                    warn!("Missing clusters: {:?}", missing_clusters);
                }
                if !extra_clusters.is_empty() {
                    warn!("Extra clusters in DB: {:?}", extra_clusters);
                }
                return Err(anyhow!(
                    "Discrepancies found in clusters for node {}",
                    db_node_id
                ));
            } else {
                debug!("All clusters verified successfully for node {}", db_node_id);
            }
        } else {
            warn!("No node found with ID: {}", node_id);
        }
        Ok(())
    }

    pub async fn create_or_update_question(
        &self,
        question: &Neo4jQuestion,
        interaction_id: &str,
    ) -> Result<String> {
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
            Err(anyhow::anyhow!("Failed to create or update question"))
        }
    }

    pub async fn create_response(
        &self,
        response: &Neo4jResponse,
        interaction_id: &str,
        model_id: &str,
    ) -> Result<String> {
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
                    .param("id", response.id.clone())
                    .param("content", response.content.clone())
                    .param("vector", BoltType::List(response.vector.clone()))
                    .param("timestamp", response.timestamp.to_rfc3339())
                    .param("confidence", response.confidence)
                    .param(
                        "llm_specific_data",
                        serde_json::to_string(&response.llm_specific_data)?,
                    )
                    .param("interaction_id", interaction_id)
                    .param("model_id", model_id),
            )
            .await?;

        if let Some(row) = result.next().await? {
            Ok(row.get("response_id")?)
        } else {
            Err(anyhow::anyhow!("Failed to create response"))
        }
    }

    async fn extract_themes_and_keywords(
        &self,
        content: &str,
        _config: &VoyageAIConfig,
    ) -> Result<(Vec<String>, Vec<String>)> {
        debug!("Extracting themes and keywords");
        debug!("content: {}", content);
        let stemmer = Stemmer::create(Algorithm::English);
        let stop_words: Vec<String> = stop_words::get(stop_words::LANGUAGE::English);

        // Tokenize and clean the content
        let words: Vec<String> = content
            .split_whitespace()
            .map(|word| word.to_lowercase())
            .filter(|word| {
                word.len() > 4 && // Filter out very short words
                    !stop_words.contains(word) && // Filter out stop words
                    word.chars().any(|c| c.is_alphabetic()) // Ensure at least one alphabetic character
            })
            .collect();

        let mut word_freq: HashMap<String, usize> = HashMap::new();

        for word in words {
            let stemmed = stemmer.stem(&word).to_string();
            *word_freq.entry(stemmed).or_insert(0) += 1;
        }

        let mut sorted_words: Vec<_> = word_freq.into_iter().collect();
        sorted_words.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

        let themes: Vec<String> = sorted_words
            .iter()
            .take(3) // Extract top 5 as themes
            .map(|(word, count)| format!("{}:{}", word, count))
            .collect();

        let keywords: Vec<String> = sorted_words
            .iter()
            .skip(5)
            .take(3) // Extract next 10 as keywords
            .map(|(word, count)| format!("{}:{}", word, count))
            .collect();

        debug!("Extracted themes: {:?}", themes);
        debug!("Extracted keywords: {:?}", keywords);

        Ok((themes, keywords))
    }

    async fn extract_clusters(
        &self,
        content: &str,
        all_documents: &[String],
    ) -> Result<Vec<String>> {
        debug!("Extracting clusters");
        let stemmer = Stemmer::create(Algorithm::English);
        let stop_words: HashSet<_> = stop_words::get(stop_words::LANGUAGE::English)
            .into_iter()
            .collect();

        // Function to tokenize and clean text
        let tokenize = |text: &str| -> Vec<String> {
            text.split_whitespace()
                .map(|word| word.to_lowercase())
                .filter(|word| {
                    word.len() > 5 && // Filter out very short words
                        !stop_words.contains(word) && // Filter out stop words
                        word.chars().any(|c| c.is_alphabetic()) // Ensure at least one alphabetic character
                })
                .map(|word| stemmer.stem(&word).to_string())
                .collect()
        };

        // Calculate TF-IDF
        let doc_words = tokenize(content);
        let mut tf: HashMap<String, f64> = HashMap::new();
        let mut df: HashMap<String, f64> = HashMap::new();
        let n_docs = all_documents.len() as f64;

        // Calculate TF for the current document
        for word in &doc_words {
            *tf.entry(word.clone()).or_insert(0.0) += 1.0;
        }

        // Normalize TF
        let doc_len = doc_words.len() as f64;
        for count in tf.values_mut() {
            *count /= doc_len;
        }

        // Calculate DF
        for doc in all_documents {
            let unique_words: HashSet<_> = tokenize(doc).into_iter().collect();
            for word in unique_words {
                *df.entry(word).or_insert(0.0) += 1.0;
            }
        }

        // Calculate TF-IDF
        let mut tfidf: Vec<(String, f64)> = tf
            .into_iter()
            .map(|(word, tf_value)| {
                let df_value = df.get(&word).unwrap_or(&1.0);
                let idf = (n_docs / df_value).ln();
                (word, tf_value * idf)
            })
            .collect();

        // Sort by TF-IDF score
        tfidf.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Extract top terms as clusters
        let clusters: Vec<String> = tfidf
            .into_iter()
            .take(3) // Take top 5 terms as clusters
            .map(|(word, score)| format!("{}:{:.2}", word, score))
            .collect();

        debug!("Extracted clusters: {:?}", clusters);
        Ok(clusters)
    }

    pub async fn execute_cypher(&self, cypher_query: &str) -> Result<Value> {
        info!("Executing Cypher query: {}", cypher_query);

        let query = query(cypher_query);

        let mut txn = self.graph.start_txn().await?;
        let mut result = txn.execute(query).await?;

        let mut rows = Vec::new();

        while let Some(row) = result.next(txn.handle()).await? {
            let row_value = self.row_to_json(&row)?;
            rows.push(row_value);
        }

        txn.commit().await?;

        Ok(json!(rows))
    }

    fn row_to_json(&self, row: &Row) -> Result<Value> {
        row.to::<Value>()
            .map_err(|e| anyhow!("Failed to convert row to JSON: {}", e))
    }

    pub async fn get_database_schema(&self) -> Result<String, Error> {
        let query = "
        CALL apoc.meta.schema()
        YIELD value
        RETURN value
        ";

        let result = self.execute_cypher(query).await?;

        // Convert the schema to a string representation
        let schema_str = serde_json::to_string_pretty(&result)?;

        Ok(schema_str)
    }
}

// Define the necessary structs
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
    pub llm_specific_data: Value,
}

#[derive(Debug, Clone)]
pub struct Neo4jModel {
    pub id: String,
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone)]
pub struct Neo4jTokenUsage {
    pub id: String,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
}

// Implement other necessary structs and methods...
