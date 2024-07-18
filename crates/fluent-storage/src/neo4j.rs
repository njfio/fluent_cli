// crates/fluent-storage/src/neo4j.rs
use neo4rs::{Graph, query};
use anyhow::Result;

pub async fn connect_neo4j(uri: &str, user: &str, password: &str) -> Result<Graph> {
    let graph = Graph::new(uri, user, password).await?;
    Ok(graph)
}

pub async fn store_data(graph: &Graph, data: &str) -> Result<()> {
    let query = query("CREATE (n:Data {content: $content})")
        .param("content", data);
    graph.run(query).await?;
    Ok(())
}
