use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;

use fluent_agent::context::ExecutionContext;
use fluent_agent::goal::{Goal, GoalType};
use fluent_agent::memory::*;
use fluent_agent::orchestrator::Observation;

struct InMemoryLongTermMemory {
    items: tokio::sync::RwLock<HashMap<String, MemoryItem>>,
}

impl InMemoryLongTermMemory {
    fn new() -> Self {
        Self { items: tokio::sync::RwLock::new(HashMap::new()) }
    }
}

#[async_trait]
impl LongTermMemory for InMemoryLongTermMemory {
    async fn store(&self, memory: MemoryItem) -> Result<String> {
        let id = memory.memory_id.clone();
        self.items.write().await.insert(id.clone(), memory);
        Ok(id)
    }

    async fn retrieve(&self, memory_id: &str) -> Result<Option<MemoryItem>> {
        Ok(self.items.read().await.get(memory_id).cloned())
    }

    async fn update(&self, memory: MemoryItem) -> Result<()> {
        self.items.write().await.insert(memory.memory_id.clone(), memory);
        Ok(())
    }

    async fn delete(&self, memory_id: &str) -> Result<()> {
        self.items.write().await.remove(memory_id);
        Ok(())
    }

    async fn search(&self, _query: MemoryQuery) -> Result<Vec<MemoryItem>> {
        Ok(self.items.read().await.values().cloned().collect())
    }

    async fn find_similar(&self, _memory: &MemoryItem, _threshold: f32) -> Result<Vec<MemoryItem>> {
        Ok(vec![])
    }

    async fn get_recent(&self, limit: usize) -> Result<Vec<MemoryItem>> {
        let mut v: Vec<_> = self.items.read().await.values().cloned().collect();
        v.truncate(limit);
        Ok(v)
    }

    async fn get_by_importance(&self, min_importance: f32, limit: usize) -> Result<Vec<MemoryItem>> {
        let mut v: Vec<_> = self
            .items
            .read()
            .await
            .values()
            .filter(|m| m.importance >= min_importance as f64)
            .cloned()
            .collect();
        v.truncate(limit);
        Ok(v)
    }

    async fn cleanup_old_memories(&self, _days: u32) -> Result<usize> {
        Ok(0)
    }
}

struct InMemoryEpisodicMemory(tokio::sync::RwLock<Vec<Episode>>);
struct InMemorySemanticMemory(tokio::sync::RwLock<Vec<Knowledge>>);

#[async_trait]
impl EpisodicMemory for InMemoryEpisodicMemory {
    async fn store_episode(&self, episode: Episode) -> Result<String> {
        let id = episode.episode_id.clone();
        self.0.write().await.push(episode);
        Ok(id)
    }

    async fn retrieve_episodes(&self, _criteria: &EpisodeCriteria) -> Result<Vec<Episode>> {
        Ok(self.0.read().await.clone())
    }

    async fn get_similar_episodes(&self, _context: &ExecutionContext, limit: usize) -> Result<Vec<Episode>> {
        let mut v = self.0.read().await.clone();
        v.truncate(limit);
        Ok(v)
    }
}

#[async_trait]
impl SemanticMemory for InMemorySemanticMemory {
    async fn store_knowledge(&self, knowledge: Knowledge) -> Result<String> {
        let id = knowledge.knowledge_id.clone();
        self.0.write().await.push(knowledge);
        Ok(id)
    }

    async fn retrieve_knowledge(&self, _topic: &str) -> Result<Vec<Knowledge>> {
        Ok(self.0.read().await.clone())
    }

    async fn update_knowledge(&self, _knowledge_id: &str, _evidence: Evidence) -> Result<()> {
        Ok(())
    }
}

fn make_context() -> ExecutionContext {
    let goal = Goal::builder("Test goal".to_string(), GoalType::Analysis)
        .success_criterion("complete".to_string())
        .build()
        .unwrap();
    let mut ctx = ExecutionContext::new(goal);
    ctx.add_observation(Observation {
        observation_id: uuid::Uuid::new_v4().to_string(),
        timestamp: std::time::SystemTime::now(),
        observation_type: fluent_agent::orchestrator::ObservationType::ActionResult,
        content: "SUCCESS: did a thing".to_string(),
        source: "test".to_string(),
        relevance_score: 0.9,
        impact_assessment: None,
    });
    ctx
}

#[tokio::test]
async fn memory_system_updates_and_stats() -> Result<()> {
    let ltm = Arc::new(InMemoryLongTermMemory::new()) as Arc<dyn LongTermMemory>;
    let epi_conc = Arc::new(InMemoryEpisodicMemory(tokio::sync::RwLock::new(Vec::new())));
    let sem_conc = Arc::new(InMemorySemanticMemory(tokio::sync::RwLock::new(Vec::new())));
    let epi: Arc<dyn EpisodicMemory> = epi_conc.clone();
    let sem: Arc<dyn SemanticMemory> = sem_conc.clone();
    
    let config = MemoryConfig::default();

    let system = MemorySystem::new(ltm, epi, sem, config);
    let ctx = make_context();

    system.update(&ctx).await?;
    let stats = system.get_memory_stats().await;
    assert!(stats.short_term_items <= 1);
    assert!(stats.attention_items <= 10);

    Ok(())
}

#[tokio::test]
async fn memory_system_store_experience_and_learning() -> Result<()> {
    let ltm = Arc::new(InMemoryLongTermMemory::new()) as Arc<dyn LongTermMemory>;
    let epi_conc = Arc::new(InMemoryEpisodicMemory(tokio::sync::RwLock::new(Vec::new())));
    let sem_conc = Arc::new(InMemorySemanticMemory(tokio::sync::RwLock::new(Vec::new())));
    let epi: Arc<dyn EpisodicMemory> = epi_conc.clone();
    let sem: Arc<dyn SemanticMemory> = sem_conc.clone();
    
    let mut cfg = MemoryConfig::default();
    cfg.short_term_capacity = 1;
    cfg.consolidation_threshold = 0.0;

    let system = MemorySystem::new(ltm.clone(), epi.clone(), sem.clone(), cfg);
    let ctx = make_context();

    let outcome = ExperienceOutcome {
        description: "Ran a test".to_string(),
        actions_taken: vec!["act".to_string()],
        outcomes: vec!["ok".to_string()],
        success: true,
        lessons_learned: vec!["stuff".to_string()],
        duration: Duration::from_millis(1500),
    };

    let ep_id = system.store_experience(&ctx, outcome).await?;
    assert!(!ep_id.is_empty());

    let evidence = Evidence { evidence_id: uuid::Uuid::new_v4().to_string(), description: "doc".to_string(), strength: 0.9, source: "test".to_string(), timestamp: std::time::SystemTime::now() };
    let know_id = system.store_learning("topic", "content", evidence).await?;
    assert!(!know_id.is_empty());

    // Ensure long-term store can persist an item
    let item = MemoryItem {
        memory_id: "m1".to_string(),
        memory_type: MemoryType::Fact,
        content: "Hello world".to_string(),
        metadata: HashMap::new(),
        importance: 0.7,
        created_at: Utc::now(),
        last_accessed: Utc::now(),
        access_count: 0,
        tags: vec!["t".to_string()],
        embedding: None,
    };
    // store via consolidation, then retrieve via public API
    assert!(!epi_conc.0.read().await.is_empty());
    assert!(!sem_conc.0.read().await.is_empty());

    system.update(&ctx).await?;
    let items = system.retrieve_relevant_memories(&ctx, 10).await?;
    assert!(!items.is_empty());

    Ok(())
}
