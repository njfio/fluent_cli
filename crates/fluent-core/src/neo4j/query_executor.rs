//! Query execution and result processing for Neo4j
//! 
//! This module handles Cypher query execution, transaction management,
//! and result processing for Neo4j database operations.

use anyhow::{anyhow, Result};
use neo4rs::{query, Graph, Row};
use serde_json::{json, Value};
use log::info;

/// Query executor for Neo4j operations
pub struct QueryExecutor<'a> {
    graph: &'a Graph,
}

impl<'a> QueryExecutor<'a> {
    pub fn new(graph: &'a Graph) -> Self {
        Self { graph }
    }

    /// Execute a Cypher query and return JSON results
    pub async fn execute_cypher(&self, cypher_query: &str) -> Result<Value> {
        info!("Executing Cypher query: {}", cypher_query);

        let query = query(cypher_query);
        let mut txn = self.graph.start_txn().await?;
        let mut result = txn.execute(query).await?;

        let rows = self.collect_rows(&mut result, &mut txn).await?;
        txn.commit().await?;

        Ok(json!(rows))
    }

    /// Execute a query with parameters
    pub async fn execute_query_with_params(
        &self,
        query: neo4rs::Query,
    ) -> Result<Vec<Row>> {
        let mut result = self.graph.execute(query).await?;
        let mut rows = Vec::new();

        while let Some(row) = result.next().await? {
            rows.push(row);
        }

        Ok(rows)
    }

    /// Execute a query in a transaction
    pub async fn execute_in_transaction<F, T>(&self, operation: F) -> Result<T>
    where
        F: FnOnce(&mut neo4rs::Txn) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send + '_>>,
    {
        let mut txn = self.graph.start_txn().await?;
        
        match operation(&mut txn).await {
            Ok(result) => {
                txn.commit().await?;
                Ok(result)
            }
            Err(e) => {
                txn.rollback().await?;
                Err(e)
            }
        }
    }

    /// Collect all rows from a result stream
    async fn collect_rows(
        &self,
        result: &mut neo4rs::RowStream,
        txn: &mut neo4rs::Txn,
    ) -> Result<Vec<Value>> {
        let mut rows = Vec::new();

        while let Some(row) = result.next(txn.handle()).await? {
            let row_value = self.row_to_json(&row)?;
            rows.push(row_value);
        }

        Ok(rows)
    }

    /// Convert a Neo4j row to JSON
    fn row_to_json(&self, row: &Row) -> Result<Value> {
        row.to::<Value>()
            .map_err(|e| anyhow!("Failed to convert row to JSON: {}", e))
    }

    /// Get database schema information
    pub async fn get_database_schema(&self) -> Result<String> {
        let query = "
        CALL apoc.meta.schema()
        YIELD value
        RETURN value
        ";

        let result = self.execute_cypher(query).await?;
        let schema_str = serde_json::to_string_pretty(&result)?;
        Ok(schema_str)
    }

    /// Execute a simple query and return the first result
    pub async fn execute_single_result(&self, cypher_query: &str) -> Result<Option<Row>> {
        let query = query(cypher_query);
        let mut result = self.graph.execute(query).await?;
        Ok(result.next().await?)
    }

    /// Execute a query and return count
    pub async fn execute_count_query(&self, cypher_query: &str) -> Result<i64> {
        let row = self.execute_single_result(cypher_query).await?
            .ok_or_else(|| anyhow!("No result returned from count query"))?;
        
        // Try different possible column names for count
        if let Ok(count) = row.get::<i64>("count") {
            Ok(count)
        } else if let Ok(count) = row.get::<i64>("total") {
            Ok(count)
        } else if let Ok(count) = row.get::<i64>("n") {
            Ok(count)
        } else {
            Err(anyhow!("Could not extract count from query result"))
        }
    }

    /// Check if a node exists
    pub async fn node_exists(&self, node_id: &str, node_type: &str) -> Result<bool> {
        let query = format!(
            "MATCH (n:{}) WHERE n.id = $node_id RETURN count(n) as count",
            node_type
        );
        
        let cypher_query = neo4rs::query(&query)
            .param("node_id", node_id);
        
        let rows = self.execute_query_with_params(cypher_query).await?;
        
        if let Some(row) = rows.first() {
            let count: i64 = row.get("count")?;
            Ok(count > 0)
        } else {
            Ok(false)
        }
    }

    /// Get node content by ID and type
    pub async fn get_node_content(&self, node_id: &str, node_type: &str) -> Result<String> {
        let query = neo4rs::query(
            "
            MATCH (n)
            WHERE (n:Document OR n:Question OR n:Response) AND n.id = $node_id
            RETURN n.content AS content
            "
        ).param("node_id", node_id);

        let rows = self.execute_query_with_params(query).await?;
        
        if let Some(row) = rows.first() {
            Ok(row.get("content")?)
        } else {
            Err(anyhow!("Node not found: {} with id {}", node_type, node_id))
        }
    }

    /// Get all documents content
    pub async fn get_all_documents(&self) -> Result<Vec<String>> {
        let query = neo4rs::query(
            "
            MATCH (n)
            WHERE (n:Document OR n:Question OR n:Response)
            RETURN n.content AS content
            "
        );

        let rows = self.execute_query_with_params(query).await?;
        let mut documents = Vec::new();

        for row in rows {
            let content: String = row.get("content")?;
            documents.push(content);
        }

        Ok(documents)
    }
}

/// Result processor for complex query results
pub struct ResultProcessor;

impl ResultProcessor {
    /// Process document statistics from query results
    pub fn process_document_statistics(rows: &[Row]) -> Result<crate::types::DocumentStatistics> {
        if let Some(row) = rows.first() {
            Ok(crate::types::DocumentStatistics {
                document_count: row.get::<i64>("document_count")?,
                avg_content_length: row.get::<f64>("avg_content_length")?,
                chunk_count: row.get::<i64>("chunk_count")?,
                embedding_count: row.get::<i64>("embedding_count")?,
            })
        } else {
            Err(anyhow!("No statistics data found"))
        }
    }

    /// Process theme and keyword results
    pub fn process_themes_keywords(rows: &[Row]) -> Result<(Vec<String>, Vec<String>)> {
        let mut themes = Vec::new();
        let mut keywords = Vec::new();

        for row in rows {
            if let Ok(theme) = row.get::<String>("theme") {
                themes.push(theme);
            }
            if let Ok(keyword) = row.get::<String>("keyword") {
                keywords.push(keyword);
            }
        }

        Ok((themes, keywords))
    }

    /// Process cluster results
    pub fn process_clusters(rows: &[Row]) -> Result<Vec<String>> {
        let mut clusters = Vec::new();

        for row in rows {
            if let Ok(cluster) = row.get::<String>("cluster") {
                clusters.push(cluster);
            }
        }

        Ok(clusters)
    }

    /// Process verification results
    pub fn process_verification_result(rows: &[Row]) -> Result<VerificationResult> {
        if let Some(row) = rows.first() {
            Ok(VerificationResult {
                node_id: row.get("node_id")?,
                expected_count: row.get::<i64>("expected_count")? as usize,
                actual_count: row.get::<i64>("actual_count")? as usize,
                missing_items: row.get::<Vec<String>>("missing_items").unwrap_or_default(),
                extra_items: row.get::<Vec<String>>("extra_items").unwrap_or_default(),
            })
        } else {
            Err(anyhow!("No verification result found"))
        }
    }
}

/// Result of verification operations
#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub node_id: String,
    pub expected_count: usize,
    pub actual_count: usize,
    pub missing_items: Vec<String>,
    pub extra_items: Vec<String>,
}

impl VerificationResult {
    /// Check if verification passed
    pub fn is_valid(&self) -> bool {
        self.missing_items.is_empty() && self.extra_items.is_empty()
    }

    /// Get verification error message if invalid
    pub fn error_message(&self) -> Option<String> {
        if self.is_valid() {
            None
        } else {
            let mut errors = Vec::new();
            
            if !self.missing_items.is_empty() {
                errors.push(format!("Missing items: {:?}", self.missing_items));
            }
            
            if !self.extra_items.is_empty() {
                errors.push(format!("Extra items: {:?}", self.extra_items));
            }
            
            Some(format!("Verification failed for node {}: {}", 
                        self.node_id, errors.join(", ")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verification_result_valid() {
        let result = VerificationResult {
            node_id: "test".to_string(),
            expected_count: 2,
            actual_count: 2,
            missing_items: vec![],
            extra_items: vec![],
        };
        
        assert!(result.is_valid());
        assert!(result.error_message().is_none());
    }

    #[test]
    fn test_verification_result_invalid() {
        let result = VerificationResult {
            node_id: "test".to_string(),
            expected_count: 2,
            actual_count: 1,
            missing_items: vec!["item1".to_string()],
            extra_items: vec![],
        };
        
        assert!(!result.is_valid());
        assert!(result.error_message().is_some());
    }
}
