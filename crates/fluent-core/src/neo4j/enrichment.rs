//! Document enrichment operations for Neo4j
//! 
//! This module handles document enrichment including themes, keywords,
//! clustering, and sentiment analysis for Neo4j stored documents.

use anyhow::{anyhow, Result};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use neo4rs::{query, BoltString, BoltType, Graph};
use log::{debug, warn, error};

use crate::neo4j_client::VoyageAIConfig;
use crate::neo4j::query_executor::QueryExecutor;

/// Configuration for document enrichment intervals
#[derive(Debug, Clone)]
pub struct EnrichmentConfig {
    pub themes_keywords_interval: ChronoDuration,
    pub clustering_interval: ChronoDuration,
    pub sentiment_interval: ChronoDuration,
}

impl Default for EnrichmentConfig {
    fn default() -> Self {
        Self {
            themes_keywords_interval: ChronoDuration::hours(1),
            clustering_interval: ChronoDuration::days(1),
            sentiment_interval: ChronoDuration::hours(1),
        }
    }
}

/// Status of enrichment operations for a node
#[derive(Debug, Clone)]
pub struct EnrichmentStatus {
    pub last_themes_keywords_update: Option<DateTime<Utc>>,
    pub last_clustering_update: Option<DateTime<Utc>>,
    pub last_sentiment_update: Option<DateTime<Utc>>,
}

/// Document enrichment manager
pub struct DocumentEnrichmentManager<'a> {
    graph: &'a Graph,
    query_executor: QueryExecutor<'a>,
    voyage_config: Option<&'a VoyageAIConfig>,
}

impl<'a> DocumentEnrichmentManager<'a> {
    pub fn new(graph: &'a Graph, voyage_config: Option<&'a VoyageAIConfig>) -> Self {
        let query_executor = QueryExecutor::new(graph);
        Self {
            graph,
            query_executor,
            voyage_config,
        }
    }

    /// Perform incremental enrichment on a document
    pub async fn enrich_document_incrementally(
        &self,
        node_id: &str,
        node_type: &str,
        config: &EnrichmentConfig,
    ) -> Result<()> {
        let status = self.get_enrichment_status(node_id, node_type).await?;
        let now = Utc::now();

        if let Some(voyage_config) = self.voyage_config {
            self.update_themes_keywords_if_needed(&status, node_id, node_type, &now, config, voyage_config).await?;
            self.update_clustering_if_needed(&status, node_id, node_type, &now, config).await?;
            self.update_sentiment_if_needed(&status, node_id, node_type, &now, config).await?;
            
            self.update_enrichment_status(node_id, node_type, &now).await?;
            Ok(())
        } else {
            Err(anyhow!("VoyageAI configuration not found"))
        }
    }

    /// Update themes and keywords if needed
    async fn update_themes_keywords_if_needed(
        &self,
        status: &EnrichmentStatus,
        node_id: &str,
        node_type: &str,
        now: &DateTime<Utc>,
        config: &EnrichmentConfig,
        voyage_config: &VoyageAIConfig,
    ) -> Result<()> {
        if status
            .last_themes_keywords_update
            .map_or(true, |last| *now - last > config.themes_keywords_interval)
        {
            self.update_themes_and_keywords(node_id, node_type, voyage_config).await?;
        }
        Ok(())
    }

    /// Update clustering if needed
    async fn update_clustering_if_needed(
        &self,
        status: &EnrichmentStatus,
        node_id: &str,
        node_type: &str,
        now: &DateTime<Utc>,
        config: &EnrichmentConfig,
    ) -> Result<()> {
        if status
            .last_clustering_update
            .map_or(true, |last| *now - last > config.clustering_interval)
        {
            self.update_clustering(node_id, node_type).await?;
        }
        Ok(())
    }

    /// Update sentiment if needed
    async fn update_sentiment_if_needed(
        &self,
        status: &EnrichmentStatus,
        node_id: &str,
        node_type: &str,
        now: &DateTime<Utc>,
        config: &EnrichmentConfig,
    ) -> Result<()> {
        if status
            .last_sentiment_update
            .map_or(true, |last| *now - last > config.sentiment_interval)
        {
            self.update_sentiment(node_id, node_type).await?;
        }
        Ok(())
    }

    /// Update themes and keywords for a node
    async fn update_themes_and_keywords(
        &self,
        node_id: &str,
        node_type: &str,
        voyage_config: &VoyageAIConfig,
    ) -> Result<()> {
        debug!("Updating themes and keywords for {} {}", node_type, node_id);
        
        let content = self.query_executor.get_node_content(node_id, node_type).await?;
        let (themes, keywords) = self.extract_themes_and_keywords(&content, voyage_config)?;
        
        self.create_theme_and_keyword_nodes(node_id, &themes, &keywords).await?;
        self.verify_themes_and_keywords(node_id, &themes, &keywords).await?;
        
        Ok(())
    }

    /// Update clustering for a node
    async fn update_clustering(&self, node_id: &str, node_type: &str) -> Result<()> {
        debug!("Updating clustering for {} {}", node_type, node_id);

        let content = self.query_executor.get_node_content(node_id, node_type).await?;
        let all_documents = self.query_executor.get_all_documents().await?;
        let clusters = self.extract_clusters(&content, &all_documents).await?;

        self.create_and_assign_clusters(node_id, &clusters).await?;
        self.verify_clusters(node_id, &clusters).await?;

        Ok(())
    }

    /// Update sentiment for a node
    async fn update_sentiment(&self, node_id: &str, node_type: &str) -> Result<()> {
        debug!("Updating sentiment for {} {}", node_type, node_id);
        
        let content = self.query_executor.get_node_content(node_id, node_type).await?;
        let sentiment = self.analyze_sentiment(&content).await?;
        
        self.create_sentiment_node(node_id, &sentiment).await?;
        
        Ok(())
    }

    /// Create theme and keyword nodes
    async fn create_theme_and_keyword_nodes(
        &self,
        node_id: &str,
        themes: &[String],
        keywords: &[String],
    ) -> Result<()> {
        debug!("Creating theme and keyword nodes for node {}", node_id);
        
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

        let rows = self.query_executor.execute_query_with_params(query).await?;
        
        if let Some(row) = rows.first() {
            let total_count: i64 = row.get("total_count")?;
            debug!("Created {} theme/keyword relationships for node {}", total_count, node_id);
        }

        Ok(())
    }

    /// Create and assign clusters
    async fn create_and_assign_clusters(&self, node_id: &str, clusters: &[String]) -> Result<()> {
        debug!("Creating and assigning cluster nodes for node {}", node_id);
        
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

        let result = self.graph.execute(query).await;
        match result {
            Ok(mut stream) => {
                if let Some(row) = stream.next().await? {
                    let total_count: i64 = row.get("total_count")?;
                    debug!("Created and assigned {} cluster nodes for node {}", total_count, node_id);
                    
                    if total_count == 0 {
                        warn!("No clusters were created or assigned for node {}", node_id);
                    }
                } else {
                    warn!("No result returned from cluster creation query for node {}", node_id);
                }
            }
            Err(e) => {
                error!("Error executing cluster creation query for node {}: {:?}", node_id, e);
                return Err(anyhow!("Failed to create and assign cluster nodes: {:?}", e));
            }
        }

        Ok(())
    }

    /// Create sentiment node
    async fn create_sentiment_node(&self, node_id: &str, sentiment: &SentimentAnalysis) -> Result<()> {
        debug!("Creating sentiment node for node {}", node_id);
        
        let query = query(
            "
            MATCH (n)
            WHERE (n:Document OR n:Question OR n:Response) AND n.id = $node_id
            MERGE (s:Sentiment {
                score: $score,
                label: $label,
                confidence: $confidence
            })
            MERGE (n)-[:HAS_SENTIMENT]->(s)
            RETURN s.score as sentiment_score
            ",
        )
        .param("node_id", BoltType::String(BoltString::from(node_id)))
        .param("score", sentiment.score)
        .param("label", sentiment.label.as_str())
        .param("confidence", sentiment.confidence);

        let rows = self.query_executor.execute_query_with_params(query).await?;
        
        if rows.is_empty() {
            return Err(anyhow!("Failed to create sentiment node"));
        }

        Ok(())
    }

    /// Verify themes and keywords
    async fn verify_themes_and_keywords(
        &self,
        node_id: &str,
        expected_themes: &[String],
        expected_keywords: &[String],
    ) -> Result<()> {
        let query = query(
            "
            MATCH (n {id: $node_id})
            OPTIONAL MATCH (n)-[:HAS_THEME]->(t:Theme)
            OPTIONAL MATCH (n)-[:HAS_KEYWORD]->(k:Keyword)
            RETURN
                n.id as node_id,
                collect(distinct t.name) as themes,
                collect(distinct k.name) as keywords
            ",
        )
        .param("node_id", BoltType::String(BoltString::from(node_id)));

        let rows = self.query_executor.execute_query_with_params(query).await?;
        
        if let Some(row) = rows.first() {
            let db_themes: Vec<String> = row.get("themes")?;
            let db_keywords: Vec<String> = row.get("keywords")?;
            
            self.validate_themes_keywords(&db_themes, &db_keywords, expected_themes, expected_keywords, node_id)?;
        }

        Ok(())
    }

    /// Verify clusters
    async fn verify_clusters(&self, node_id: &str, expected_clusters: &[String]) -> Result<()> {
        let query = query(
            "
            MATCH (n {id: $node_id})
            OPTIONAL MATCH (n)-[:BELONGS_TO]->(c:Cluster)
            RETURN
                n.id as node_id,
                collect(distinct c.name) as clusters
            ",
        )
        .param("node_id", BoltType::String(BoltString::from(node_id)));

        let rows = self.query_executor.execute_query_with_params(query).await?;
        
        if let Some(row) = rows.first() {
            let db_clusters: Vec<String> = row.get("clusters")?;
            self.validate_clusters(&db_clusters, expected_clusters, node_id)?;
        }

        Ok(())
    }

    /// Validate themes and keywords
    fn validate_themes_keywords(
        &self,
        db_themes: &[String],
        db_keywords: &[String],
        expected_themes: &[String],
        expected_keywords: &[String],
        node_id: &str,
    ) -> Result<()> {
        let missing_themes: Vec<_> = expected_themes
            .iter()
            .filter(|t| !db_themes.contains(t))
            .cloned()
            .collect();
        
        let missing_keywords: Vec<_> = expected_keywords
            .iter()
            .filter(|k| !db_keywords.contains(k))
            .cloned()
            .collect();

        if !missing_themes.is_empty() || !missing_keywords.is_empty() {
            warn!("Discrepancies found for node {}:", node_id);
            if !missing_themes.is_empty() {
                warn!("Missing themes: {:?}", missing_themes);
            }
            if !missing_keywords.is_empty() {
                warn!("Missing keywords: {:?}", missing_keywords);
            }
        } else {
            debug!("All themes and keywords verified successfully for node {}", node_id);
        }

        Ok(())
    }

    /// Validate clusters
    fn validate_clusters(&self, db_clusters: &[String], expected_clusters: &[String], node_id: &str) -> Result<()> {
        let missing_clusters: Vec<_> = expected_clusters
            .iter()
            .filter(|c| !db_clusters.contains(c))
            .cloned()
            .collect();

        if !missing_clusters.is_empty() {
            warn!("Missing clusters for node {}: {:?}", node_id, missing_clusters);
        } else {
            debug!("All clusters verified successfully for node {}", node_id);
        }

        Ok(())
    }

    // Placeholder methods for actual AI operations
    fn extract_themes_and_keywords(&self, _content: &str, _voyage_config: &VoyageAIConfig) -> Result<(Vec<String>, Vec<String>)> {
        // TODO: Implement actual theme and keyword extraction
        Ok((vec!["theme1".to_string()], vec!["keyword1".to_string()]))
    }

    async fn extract_clusters(&self, _content: &str, _all_documents: &[String]) -> Result<Vec<String>> {
        // TODO: Implement actual clustering
        Ok(vec!["cluster1".to_string()])
    }

    async fn analyze_sentiment(&self, _content: &str) -> Result<SentimentAnalysis> {
        // TODO: Implement actual sentiment analysis
        Ok(SentimentAnalysis {
            score: 0.5,
            label: "neutral".to_string(),
            confidence: 0.8,
        })
    }

    async fn get_enrichment_status(&self, _node_id: &str, _node_type: &str) -> Result<EnrichmentStatus> {
        // TODO: Implement actual status retrieval
        Ok(EnrichmentStatus {
            last_themes_keywords_update: None,
            last_clustering_update: None,
            last_sentiment_update: None,
        })
    }

    async fn update_enrichment_status(&self, _node_id: &str, _node_type: &str, _now: &DateTime<Utc>) -> Result<()> {
        // TODO: Implement actual status update
        Ok(())
    }
}

/// Sentiment analysis result
#[derive(Debug, Clone)]
pub struct SentimentAnalysis {
    pub score: f64,
    pub label: String,
    pub confidence: f64,
}
