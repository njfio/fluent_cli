use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

use crate::action::{self as act, ActionResult};
use crate::context::ExecutionContext;
use crate::observation::{ObservationProcessor, ProcessingCapability};
use crate::orchestrator::{Observation, ObservationType};
use crate::tools::ToolRegistry;
use crate::production_mcp::{ProductionMcpManager, ProductionMcpClientManager, ExecutionPreferences};
use fluent_core::traits::Engine;
use fluent_core::types::Request;
use std::pin::Pin;
use std::collections::HashMap as StdHashMap;
use crate::orchestrator::ActionType;

// -----------------------------------------------------------------------------
// Composite planner that routes to specialized planners or a base planner
// -----------------------------------------------------------------------------

pub struct CompositePlanner {
    base: Box<dyn act::ActionPlanner>,
    research: ResearchPlanner,
    longform: LongFormWriterPlanner,
    tool_registry: std::sync::Arc<ToolRegistry>,
}

impl CompositePlanner {
    pub fn new(base: Box<dyn act::ActionPlanner>) -> Self {
        // Backward-compatible constructor without registry; use empty registry
        Self { base, research: ResearchPlanner, longform: LongFormWriterPlanner, tool_registry: std::sync::Arc::new(ToolRegistry::new()) }
    }

    pub fn new_with_registry(base: Box<dyn act::ActionPlanner>, tool_registry: std::sync::Arc<ToolRegistry>) -> Self {
        Self { base, research: ResearchPlanner, longform: LongFormWriterPlanner, tool_registry }
    }

    fn choose(&self, context: &ExecutionContext) -> PlannerKind {
        let goal = context.get_current_goal().map(|g| g.description.to_lowercase()).unwrap_or_default();
        if goal.contains("research") || goal.contains("study") || goal.contains("investigate") {
            PlannerKind::Research
        } else if goal.contains("100k") || goal.contains("long-form") || goal.contains("long form") || goal.contains("book") || goal.contains("novel") || goal.contains("write a 100k") {
            PlannerKind::LongForm
        } else {
            PlannerKind::Base
        }
    }

    fn available_tool_names(&self) -> Vec<String> {
        self.tool_registry
            .get_all_available_tools()
            .into_iter()
            .map(|t| t.name.to_lowercase())
            .collect()
    }

    fn has_recent_search(&self, context: &ExecutionContext) -> bool {
        context
            .observations
            .iter()
            .rev()
            .take(10)
            .any(|o| {
                let c = o.content.to_lowercase();
                c.contains("search") || c.contains("results") || c.contains("browse")
            })
    }
}

enum PlannerKind { Base, Research, LongForm }

#[async_trait]
impl act::ActionPlanner for CompositePlanner {
    async fn plan_action(&self, reasoning: crate::orchestrator::ReasoningResult, context: &ExecutionContext) -> anyhow::Result<act::ActionPlan> {
        match self.choose(context) {
            PlannerKind::Research => {
                // Prefer MCP/registry tools that look like search/browse for first step
                let names = self.available_tool_names();
                let maybe_search = names.into_iter().find(|n| n.contains("search") || n.contains("browse") || n.contains("web"));
                if let Some(tool_name) = maybe_search {
                    if !self.has_recent_search(context) {
                        let goal = context.get_current_goal().map(|g| g.description.clone()).unwrap_or_default();
                        let mut params = StdHashMap::new();
                        params.insert("tool_name".to_string(), serde_json::json!(tool_name));
                        params.insert("query".to_string(), serde_json::json!(goal));
                        return Ok(act::ActionPlan {
                            action_id: uuid::Uuid::new_v4().to_string(),
                            action_type: ActionType::ToolExecution,
                            description: "Perform initial research search".to_string(),
                            parameters: params,
                            expected_outcome: "Search results obtained".to_string(),
                            confidence_score: 0.7,
                            estimated_duration: None,
                            risk_level: act::RiskLevel::Low,
                            alternatives: Vec::new(),
                            prerequisites: Vec::new(),
                            success_criteria: vec!["observation_contains:results".to_string()],
                        });
                    }
                }
                self.research.plan_action(reasoning, context).await
            },
            PlannerKind::LongForm => self.longform.plan_action(reasoning, context).await,
            PlannerKind::Base => self.base.plan_action(reasoning, context).await,
        }
    }

    fn get_capabilities(&self) -> Vec<act::PlanningCapability> {
        let mut caps = self.base.get_capabilities();
        caps.push(act::PlanningCapability::ResourceAllocation);
        caps.push(act::PlanningCapability::AlternativePlanning);
        caps
    }

    fn can_plan(&self, _action_type: &ActionType) -> bool { true }
}

// -----------------------------------------------------------------------------
// Research Planner
// -----------------------------------------------------------------------------

pub struct ResearchPlanner;

impl ResearchPlanner {
    fn research_dir(&self) -> String {
        std::env::var("FLUENT_RESEARCH_OUTPUT_DIR").unwrap_or_else(|_| "docs/research".to_string())
    }
    fn count_writes(&self, context: &ExecutionContext, file: &str) -> usize {
        context
            .observations
            .iter()
            .filter(|o| o.content.contains("Successfully wrote to") && o.content.contains(file))
            .count()
    }

    fn latest_output(&self, context: &ExecutionContext) -> String {
        context.get_latest_observation().map(|o| o.content).unwrap_or_default()
    }
}

#[async_trait]
impl act::ActionPlanner for ResearchPlanner {
    async fn plan_action(&self, _reasoning: crate::orchestrator::ReasoningResult, context: &ExecutionContext) -> anyhow::Result<act::ActionPlan> {
        let goal = context.get_current_goal().map(|g| g.description.clone()).unwrap_or_default();
        let base = self.research_dir();

        if self.count_writes(context, &format!("{}/outline.md", base)) == 0 {
            let mut params = StdHashMap::new();
            params.insert("tool_name".to_string(), serde_json::json!("research_generate_outline"));
            params.insert("goal".to_string(), serde_json::json!(goal));
            params.insert("out_path".to_string(), serde_json::json!(format!("{}/outline.md", base)));
            return Ok(act::ActionPlan { action_id: uuid::Uuid::new_v4().to_string(), action_type: ActionType::ToolExecution, description: "Research: generate and save outline".to_string(), parameters: params, expected_outcome: "Outline saved".to_string(), confidence_score: 0.75, estimated_duration: None, risk_level: act::RiskLevel::Low, alternatives: Vec::new(), prerequisites: Vec::new(), success_criteria: vec![format!("file_exists:{}/outline.md", base)], });
        }
        if self.count_writes(context, &format!("{}/notes.md", base)) == 0 {
            let mut params = StdHashMap::new();
            params.insert("tool_name".to_string(), serde_json::json!("research_generate_notes"));
            params.insert("goal".to_string(), serde_json::json!(goal));
            params.insert("out_path".to_string(), serde_json::json!(format!("{}/notes.md", base)));
            return Ok(act::ActionPlan { action_id: uuid::Uuid::new_v4().to_string(), action_type: ActionType::ToolExecution, description: "Research: generate and save notes".to_string(), parameters: params, expected_outcome: "Notes saved".to_string(), confidence_score: 0.7, estimated_duration: None, risk_level: act::RiskLevel::Low, alternatives: Vec::new(), prerequisites: Vec::new(), success_criteria: vec![format!("file_exists:{}/notes.md", base)], });
        }
        if self.count_writes(context, &format!("{}/summary.md", base)) == 0 {
            let mut params = StdHashMap::new();
            params.insert("tool_name".to_string(), serde_json::json!("research_generate_summary"));
            params.insert("goal".to_string(), serde_json::json!(goal));
            params.insert("out_path".to_string(), serde_json::json!(format!("{}/summary.md", base)));
            return Ok(act::ActionPlan { action_id: uuid::Uuid::new_v4().to_string(), action_type: ActionType::ToolExecution, description: "Research: generate and save summary".to_string(), parameters: params, expected_outcome: "Summary saved".to_string(), confidence_score: 0.7, estimated_duration: None, risk_level: act::RiskLevel::Low, alternatives: Vec::new(), prerequisites: Vec::new(), success_criteria: vec![format!("file_exists:{}/summary.md", base)], });
        }
        Ok(act::ActionPlan { action_id: uuid::Uuid::new_v4().to_string(), action_type: ActionType::Planning, description: "Research plan completed; awaiting success checks".to_string(), parameters: StdHashMap::new(), expected_outcome: "No-op".to_string(), confidence_score: 0.9, estimated_duration: None, risk_level: act::RiskLevel::Low, alternatives: Vec::new(), prerequisites: Vec::new(), success_criteria: vec![ format!("file_exists:{}/outline.md", base), format!("file_exists:{}/notes.md", base), format!("file_exists:{}/summary.md", base), ], })
    }
    fn get_capabilities(&self) -> Vec<act::PlanningCapability> { vec![act::PlanningCapability::ToolSelection] }
    fn can_plan(&self, _action_type: &ActionType) -> bool { true }
}
pub struct LongFormWriterPlanner;

impl LongFormWriterPlanner {
    fn book_dir(&self) -> String {
        std::env::var("FLUENT_BOOK_OUTPUT_DIR").unwrap_or_else(|_| "docs/book".to_string())
    }
    fn target_chapters(&self) -> usize {
        std::env::var("FLUENT_BOOK_CHAPTERS").ok().and_then(|s| s.parse::<usize>().ok()).unwrap_or(10)
    }
    fn count_written_chapters(&self, context: &ExecutionContext) -> usize {
        context
            .observations
            .iter()
            .filter(|o| o.content.contains("Successfully wrote to") && o.content.contains("/ch_"))
            .count()
    }

    fn latest_output(&self, context: &ExecutionContext) -> String {
        context.get_latest_observation().map(|o| o.content).unwrap_or_default()
    }
}

#[async_trait]
impl act::ActionPlanner for LongFormWriterPlanner {
    async fn plan_action(&self, _reasoning: crate::orchestrator::ReasoningResult, context: &ExecutionContext) -> anyhow::Result<act::ActionPlan> {
        let goal = context.get_current_goal().map(|g| g.description.clone()).unwrap_or_default();

        // Stage 1: produce outline
        let base = self.book_dir();
        let outline_written = context
            .observations
            .iter()
            .any(|o| o.content.contains(&format!("Successfully wrote to {}/outline.md", base)));
        if !outline_written {
            let mut params = StdHashMap::new();
            params.insert("tool_name".to_string(), serde_json::json!("generate_book_outline"));
            params.insert("goal".to_string(), serde_json::json!(goal));
            params.insert("out_path".to_string(), serde_json::json!(format!("{}/outline.md", base)));
            return Ok(act::ActionPlan {
                action_id: uuid::Uuid::new_v4().to_string(),
                action_type: ActionType::ToolExecution,
                description: "LongForm: generate and save outline".to_string(),
                parameters: params,
                expected_outcome: "Outline saved".to_string(),
                confidence_score: 0.75,
                estimated_duration: None,
                risk_level: act::RiskLevel::Low,
                alternatives: Vec::new(),
                prerequisites: Vec::new(),
                success_criteria: vec![format!("file_exists:{}/outline.md", base)],
            });
        }

        // Stage 2: write chapters iteratively
        let written = self.count_written_chapters(context);
        let target_chapters = self.target_chapters();
        if written < target_chapters {
            // Alternate: generate chapter content or write it depending on last step
            if self.latest_output(context).to_lowercase().contains("successfully wrote to docs/book/ch_") {
                // Generate next chapter
                let chapter_num = written + 1;
                let mut params = StdHashMap::new();
                let spec = format!(
                    "Write Chapter {} of a ~100k word work (target 5k-10k for this chapter). Use the outline at docs/book/outline.md (assume known). Include headings and coherent narrative.",
                    chapter_num
                );
                params.insert("specification".to_string(), serde_json::json!(spec));
                return Ok(act::ActionPlan {
                    action_id: uuid::Uuid::new_v4().to_string(),
                    action_type: ActionType::CodeGeneration,
                    description: format!("Generate content for Chapter {}", chapter_num),
                    parameters: params,
                    expected_outcome: "Chapter draft generated".to_string(),
                    confidence_score: 0.6,
                    estimated_duration: None,
                    risk_level: act::RiskLevel::Medium,
                    alternatives: Vec::new(),
                    prerequisites: Vec::new(),
                    success_criteria: vec!["Chapter text present".to_string()],
                });
            } else {
                // Persist the latest generated content to chapter file
                let chapter_num = written + 1;
                let path = format!("{}/ch_{:02}.md", base, chapter_num);
                let mut params = StdHashMap::new();
                params.insert("operation".to_string(), serde_json::json!("write"));
                params.insert("path".to_string(), serde_json::json!(path));
                params.insert("content".to_string(), serde_json::json!(self.latest_output(context)));
                return Ok(act::ActionPlan {
                    action_id: uuid::Uuid::new_v4().to_string(),
                    action_type: ActionType::FileOperation,
                    description: format!("Persist Chapter {}", chapter_num),
                    parameters: params,
                    expected_outcome: "Chapter saved".to_string(),
                    confidence_score: 0.8,
                    estimated_duration: None,
                    risk_level: act::RiskLevel::Low,
                    alternatives: Vec::new(),
                    prerequisites: Vec::new(),
                    success_criteria: vec![format!("non_empty_file:{}/ch_01.md", base)],
                });
            }
        }

        // Stage 3: ensure index exists
        let mut params = StdHashMap::new();
        params.insert("operation".to_string(), serde_json::json!("write"));
        params.insert("path".to_string(), serde_json::json!(format!("{}/index.md", base)));
        params.insert("content".to_string(), serde_json::json!("# Book Index\n\nSee outline.md and ch_01.md ... ch_10.md"));
        let _index_plan = act::ActionPlan {
            action_id: uuid::Uuid::new_v4().to_string(),
            action_type: ActionType::FileOperation,
            description: "Write simple index for the book".to_string(),
            parameters: params,
            expected_outcome: "Index saved".to_string(),
            confidence_score: 0.9,
            estimated_duration: None,
            risk_level: act::RiskLevel::Low,
            alternatives: Vec::new(),
            prerequisites: Vec::new(),
            success_criteria: vec![
                format!("file_exists:{}/index.md", base),
                format!("file_exists:{}/outline.md", base),
            ],
        };

        // Stage 4: ensure TOC is generated and written
        let toc_written = context
            .observations
            .iter()
            .any(|o| o.content.contains(&format!("Successfully wrote to {}/toc.md", base)));
        if !toc_written {
            // If the latest output doesn't look like a TOC, generate one; otherwise write it
            let latest = self.latest_output(context);
            if !latest.to_lowercase().contains("table of contents") && !latest.contains("[TOC]") {
                let mut params = StdHashMap::new();
                params.insert("tool_name".to_string(), serde_json::json!("generate_toc"));
                params.insert("outline_path".to_string(), serde_json::json!(format!("{}/outline.md", base)));
                params.insert("chapters".to_string(), serde_json::json!(target_chapters as u64));
                params.insert("out_path".to_string(), serde_json::json!(format!("{}/toc.md", base)));
                return Ok(act::ActionPlan {
                    action_id: uuid::Uuid::new_v4().to_string(),
                    action_type: ActionType::ToolExecution,
                    description: "Generate TOC for the book".to_string(),
                    parameters: params,
                    expected_outcome: "TOC saved".to_string(),
                    confidence_score: 0.75,
                    estimated_duration: None,
                    risk_level: act::RiskLevel::Low,
                    alternatives: Vec::new(),
                    prerequisites: Vec::new(),
                    success_criteria: vec![format!("file_exists:{}/toc.md", base)],
                });
            } else {
                let mut params = StdHashMap::new();
                params.insert("operation".to_string(), serde_json::json!("write"));
                params.insert("path".to_string(), serde_json::json!(format!("{}/toc.md", base)));
                params.insert("content".to_string(), serde_json::json!(latest));
                return Ok(act::ActionPlan {
                    action_id: uuid::Uuid::new_v4().to_string(),
                    action_type: ActionType::FileOperation,
                    description: "Persist TOC".to_string(),
                    parameters: params,
                    expected_outcome: "TOC saved".to_string(),
                    confidence_score: 0.85,
                    estimated_duration: None,
                    risk_level: act::RiskLevel::Low,
                    alternatives: Vec::new(),
                    prerequisites: Vec::new(),
                    success_criteria: vec![format!("file_exists:{}/toc.md", base)],
                });
            }
        }

        // Stage 5: assemble book.md by concatenating TOC and chapters
        let mut concat_params = StdHashMap::new();
        let mut paths: Vec<String> = Vec::new();
        paths.push(format!("{}/toc.md", base));
        for i in 1..=target_chapters { paths.push(format!("{}/ch_{:02}.md", base, i)); }
        concat_params.insert("paths".to_string(), serde_json::json!(paths));
        concat_params.insert("dest".to_string(), serde_json::json!(format!("{}/book.md", base)));
        concat_params.insert("separator".to_string(), serde_json::json!("\n\n---\n\n"));

        Ok(act::ActionPlan {
            action_id: uuid::Uuid::new_v4().to_string(),
            action_type: ActionType::ToolExecution,
            description: "Assemble book.md (TOC + chapters)".to_string(),
            parameters: {
                let mut m = StdHashMap::new();
                m.insert("tool_name".to_string(), serde_json::json!("concat_files"));
                m.insert("paths".to_string(), concat_params.get("paths").unwrap().clone());
                m.insert("dest".to_string(), concat_params.get("dest").unwrap().clone());
                m.insert("separator".to_string(), concat_params.get("separator").unwrap().clone());
                m
            },
            expected_outcome: "book.md assembled".to_string(),
            confidence_score: 0.9,
            estimated_duration: None,
            risk_level: act::RiskLevel::Low,
            alternatives: Vec::new(),
            prerequisites: vec![format!("file_exists:{}/index.md", base)],
            success_criteria: vec![format!("file_exists:{}/book.md", base), format!("non_empty_file:{}/book.md", base)],
        })
    }

    fn get_capabilities(&self) -> Vec<act::PlanningCapability> { vec![act::PlanningCapability::ResourceAllocation, act::PlanningCapability::AlternativePlanning] }
    fn can_plan(&self, _action_type: &ActionType) -> bool { true }
}

// -----------------------------------------------------------------------------
// MCP Tool Executor registered into ToolRegistry
// -----------------------------------------------------------------------------

pub struct McpRegistryExecutor {
    client_mgr: std::sync::Arc<ProductionMcpClientManager>,
}

impl McpRegistryExecutor {
    pub fn new(manager: std::sync::Arc<ProductionMcpManager>) -> Self {
        Self { client_mgr: manager.client_manager() }
    }
}

#[async_trait]
impl crate::tools::ToolExecutor for McpRegistryExecutor {
    async fn execute_tool(
        &self,
        tool_name: &str,
        parameters: &std::collections::HashMap<String, serde_json::Value>,
    ) -> anyhow::Result<String> {
        let prefs = ExecutionPreferences::default();
        let params = serde_json::to_value(parameters)?;
        let result = self
            .client_mgr
            .execute_tool_with_failover(tool_name, params, prefs)
            .await?;
        // Try to extract human-readable text from result.content
        if let Ok(val) = serde_json::to_value(&result) {
            if let Some(arr) = val.get("content").and_then(|c| c.as_array()) {
                let mut out = String::new();
                for item in arr {
                    if let Some(t) = item.get("text").and_then(|x| x.as_str()) {
                        out.push_str(t);
                        out.push('\n');
                    }
                }
                if !out.trim().is_empty() {
                    return Ok(out);
                }
            }
        }
        // Fallback to JSON
        Ok(serde_json::to_string_pretty(&result)?)
    }

    fn get_available_tools(&self) -> Vec<String> {
        futures::executor::block_on(async {
            let all = self.client_mgr.get_all_tools().await;
            let mut set = std::collections::BTreeSet::new();
            for (_server, tools) in all { for t in tools { set.insert(t.name.to_string()); } }
            set.into_iter().collect()
        })
    }

    fn get_tool_description(&self, tool_name: &str) -> Option<String> {
        // Query servers to find description
        let name = tool_name.to_string();
        let desc = futures::executor::block_on(async {
            let all = self.client_mgr.get_all_tools().await;
            for (_server, tools) in all { for t in tools { if t.name == name { if let Some(d) = t.description { return Some(d.to_string()); } } } }
            None
        });
        desc.or_else(|| Some("MCP tool".to_string()))
    }

    fn validate_tool_request(
        &self,
        _tool_name: &str,
        _parameters: &std::collections::HashMap<String, serde_json::Value>,
    ) -> anyhow::Result<()> {
        // Basic pass-through validation; MCP server handles schema
        Ok(())
    }
}

/// Adapter to expose ToolRegistry via the action::ToolExecutor trait
pub struct RegistryToolAdapter {
    registry: Arc<ToolRegistry>,
}

impl RegistryToolAdapter {
    pub fn new(registry: Arc<ToolRegistry>) -> Self { Self { registry } }
}

#[async_trait]
impl act::ToolExecutor for RegistryToolAdapter {
    async fn execute_tool(
        &self,
        tool_name: &str,
        parameters: &HashMap<String, serde_json::Value>,
    ) -> Result<String> {
        self.registry.execute_tool(tool_name, parameters).await
    }

    fn get_available_tools(&self) -> Vec<String> {
        self.registry
            .get_all_available_tools()
            .into_iter()
            .map(|t| t.name)
            .collect()
    }
}

/// Simple LLM-backed code generator
pub struct LlmCodeGenerator {
    engine: Arc<Box<dyn Engine>>,
}

impl LlmCodeGenerator {
    pub fn new(engine: Arc<Box<dyn Engine>>) -> Self { Self { engine } }
}

#[async_trait]
impl act::CodeGenerator for LlmCodeGenerator {
    async fn generate_code(&self, specification: &str, _context: &ExecutionContext) -> Result<String> {
        let prompt = format!(
            "You are a senior engineer. Generate code meeting this specification.\n\nSpecification:\n{}\n\nReturn only the complete code in a single fenced block.",
            specification
        );
        let req = Request { flowname: "codegen".to_string(), payload: prompt };
        let resp = Pin::from(self.engine.execute(&req)).await?;
        Ok(resp.content)
    }

    fn get_supported_languages(&self) -> Vec<String> {
        vec!["rust".to_string(), "javascript".to_string(), "html".to_string()]
    }
}

/// Basic async filesystem manager
pub struct FsFileManager;

#[async_trait]
impl act::FileManager for FsFileManager {
    async fn read_file(&self, path: &str) -> Result<String> {
        Ok(tokio::fs::read_to_string(path).await?)
    }
    async fn write_file(&self, path: &str, content: &str) -> Result<()> {
        if let Some(parent) = std::path::Path::new(path).parent() {
            if !parent.exists() { tokio::fs::create_dir_all(parent).await?; }
        }
        tokio::fs::write(path, content).await.map_err(Into::into)
    }
    async fn create_directory(&self, path: &str) -> Result<()> {
        tokio::fs::create_dir_all(path).await.map_err(Into::into)
    }
    async fn delete_file(&self, path: &str) -> Result<()> {
        if std::path::Path::new(path).exists() {
            tokio::fs::remove_file(path).await?;
        }
        Ok(())
    }
}

/// Minimal risk assessor
pub struct SimpleRiskAssessor;

#[async_trait]
impl act::RiskAssessor for SimpleRiskAssessor {
    async fn assess_risk(&self, plan: &act::ActionPlan, _ctx: &ExecutionContext) -> Result<act::RiskLevel> {
        use crate::orchestrator::ActionType::*;
        let level = match plan.action_type {
            FileOperation => act::RiskLevel::Low,
            ToolExecution => act::RiskLevel::Low,
            Analysis => act::RiskLevel::Low,
            CodeGeneration => act::RiskLevel::Medium,
            Communication | Planning => act::RiskLevel::Low,
        };
        Ok(level)
    }
}

/// Simple observation processor that wraps action results
pub struct SimpleObservationProcessor;

#[async_trait]
impl ObservationProcessor for SimpleObservationProcessor {
    async fn process(&self, action_result: ActionResult, _context: &ExecutionContext) -> Result<Observation> {
        let content = if let Some(out) = &action_result.output { out.clone() } else { action_result.error.clone().unwrap_or_default() };
        Ok(Observation {
            observation_id: uuid::Uuid::new_v4().to_string(),
            timestamp: std::time::SystemTime::now(),
            observation_type: ObservationType::ActionResult,
            content,
            source: format!("{:?}", action_result.action_type),
            relevance_score: if action_result.success { 0.9 } else { 0.4 },
            impact_assessment: None,
        })
    }

    async fn process_environment_change(&self, change: crate::observation::EnvironmentChange, _context: &ExecutionContext) -> Result<Observation> {
        Ok(Observation {
            observation_id: uuid::Uuid::new_v4().to_string(),
            timestamp: std::time::SystemTime::now(),
            observation_type: ObservationType::EnvironmentChange,
            content: change.description,
            source: format!("{:?}", change.change_type),
            relevance_score: 0.5,
            impact_assessment: None,
        })
    }

    fn get_capabilities(&self) -> Vec<ProcessingCapability> {
        vec![
            ProcessingCapability::ActionResultAnalysis,
            ProcessingCapability::EnvironmentMonitoring,
            ProcessingCapability::LearningExtraction,
        ]
    }
}

/// Simple heuristic planner that alternates between codegen and file write
pub struct SimpleHeuristicPlanner;

#[async_trait]
impl act::ActionPlanner for SimpleHeuristicPlanner {
    async fn plan_action(&self, _reasoning: crate::orchestrator::ReasoningResult, context: &ExecutionContext) -> Result<act::ActionPlan> {
        use crate::orchestrator::ActionType;

        let goal_desc = context.get_current_goal().map(|g| g.description.clone()).unwrap_or_default();
        let iteration = context.iteration_count();

        if iteration == 0 || context.get_latest_observation().is_none() {
            // First step: generate code from the goal specification
            let mut params = HashMap::new();
            let spec = format!("Create a complete solution for: {}\nReturn a single self-contained artifact (prefer a single HTML file with embedded JS/CSS if applicable).", goal_desc);
            params.insert("specification".to_string(), serde_json::json!(spec));

            Ok(act::ActionPlan {
                action_id: uuid::Uuid::new_v4().to_string(),
                action_type: ActionType::CodeGeneration,
                description: "Generate initial code from goal".to_string(),
                parameters: params,
                expected_outcome: "Code generated".to_string(),
                confidence_score: 0.6,
                estimated_duration: None,
                risk_level: act::RiskLevel::Medium,
                alternatives: Vec::new(),
                prerequisites: Vec::new(),
                success_criteria: vec!["Non-trivial code produced".to_string()],
            })
        } else {
            // Next: persist the generated code to a file
            let output = context.get_latest_observation().map(|o| o.content).unwrap_or_default();
            let mut path = if goal_desc.to_lowercase().contains("html") || goal_desc.to_lowercase().contains("javascript") || goal_desc.to_lowercase().contains("web") {
                "examples/agent_output.html".to_string()
            } else {
                "examples/agent_output.txt".to_string()
            };
            if goal_desc.to_lowercase().contains("tetris") { path = "examples/web_tetris.html".to_string(); }
            if goal_desc.to_lowercase().contains("snake") { path = "examples/web_snake.html".to_string(); }

            let mut params = HashMap::new();
            params.insert("operation".to_string(), serde_json::json!("write"));
            params.insert("path".to_string(), serde_json::json!(path));
            params.insert("content".to_string(), serde_json::json!(output));

            Ok(act::ActionPlan {
                action_id: uuid::Uuid::new_v4().to_string(),
                action_type: ActionType::FileOperation,
                description: "Write generated artifact to file".to_string(),
                parameters: params,
                expected_outcome: "File written".to_string(),
                confidence_score: 0.7,
                estimated_duration: None,
                risk_level: act::RiskLevel::Low,
                alternatives: Vec::new(),
                prerequisites: Vec::new(),
                success_criteria: vec!["File exists with non-trivial content".to_string()],
            })
        }
    }

    fn get_capabilities(&self) -> Vec<act::PlanningCapability> {
        vec![act::PlanningCapability::ToolSelection, act::PlanningCapability::AlternativePlanning]
    }

    fn can_plan(&self, _action_type: &crate::orchestrator::ActionType) -> bool { true }
}

/// In-memory episodic memory stub (placeholder for legacy compatibility)
pub struct EpisodicMemoryStub {
    items: tokio::sync::RwLock<Vec<String>>
}

impl EpisodicMemoryStub {
    pub fn new() -> Self {
        Self {
            items: tokio::sync::RwLock::new(Vec::new())
        }
    }
    
    pub async fn store_item(&self, item: String) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        self.items.write().await.push(item);
        Ok(id)
    }
    
    pub async fn get_items(&self) -> Result<Vec<String>> {
        Ok(self.items.read().await.clone())
    }
}

/// In-memory semantic memory stub (placeholder for legacy compatibility)
pub struct SemanticMemoryStub {
    items: tokio::sync::RwLock<Vec<String>>
}

impl SemanticMemoryStub {
    pub fn new() -> Self {
        Self {
            items: tokio::sync::RwLock::new(Vec::new())
        }
    }
    
    pub async fn store_knowledge(&self, knowledge: String) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        self.items.write().await.push(knowledge);
        Ok(id)
    }
    
    pub async fn get_knowledge(&self) -> Result<Vec<String>> {
        Ok(self.items.read().await.clone())
    }
}

// -----------------------------------------------------------------------------
// Dry-run executor: simulates results, no side effects
// -----------------------------------------------------------------------------
pub struct DryRunActionExecutor;

#[async_trait]
impl act::ActionExecutor for DryRunActionExecutor {
    async fn execute(
        &self,
        plan: act::ActionPlan,
        _context: &mut ExecutionContext,
    ) -> anyhow::Result<act::ActionResult> {
        let msg = format!(
            "DRY-RUN: would execute {:?} with parameters {:?}",
            plan.action_type, plan.parameters
        );
        Ok(act::ActionResult {
            action_id: plan.action_id,
            action_type: plan.action_type,
            parameters: plan.parameters,
            result: crate::orchestrator::ActionResult {
                success: true,
                output: Some(msg.clone()),
                error: None,
                metadata: std::collections::HashMap::new(),
            },
            execution_time: std::time::Duration::from_millis(1),
            success: true,
            output: Some(msg),
            error: None,
            metadata: std::collections::HashMap::new(),
            side_effects: Vec::new(),
        })
    }

    fn get_capabilities(&self) -> Vec<act::ExecutionCapability> {
        vec![
            act::ExecutionCapability::ToolExecution,
            act::ExecutionCapability::CodeGeneration,
            act::ExecutionCapability::FileOperations,
            act::ExecutionCapability::DataAnalysis,
            act::ExecutionCapability::ErrorRecovery,
            act::ExecutionCapability::NetworkRequests,
            act::ExecutionCapability::ProcessManagement,
        ]
    }

    fn can_execute(&self, _action_type: &crate::orchestrator::ActionType) -> bool { true }
}
