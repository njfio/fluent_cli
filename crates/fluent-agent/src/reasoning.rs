use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use std::pin::Pin;
use std::sync::Arc;

use crate::context::ExecutionContext;
use crate::orchestrator::{ReasoningResult, ReasoningType};
use fluent_core::traits::Engine;
use fluent_core::types::{Request, Usage, Cost};
use fluent_core::neo4j_client::Neo4jClient;

/// Trait for reasoning engines that can analyze context and plan actions
#[async_trait]
pub trait ReasoningEngine: Send + Sync {
    /// Analyze the current execution context and generate reasoning output
    async fn reason(&self, context: &ExecutionContext) -> Result<ReasoningResult>;
    
    /// Get the reasoning capabilities of this engine
    fn get_capabilities(&self) -> Vec<ReasoningCapability>;
    
    /// Validate if this engine can handle the given reasoning type
    fn can_handle(&self, reasoning_type: &ReasoningType) -> bool;
}

/// Capabilities that a reasoning engine can provide
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReasoningCapability {
    GoalDecomposition,
    TaskPlanning,
    ProblemSolving,
    ContextAnalysis,
    StrategyFormulation,
    SelfReflection,
    ErrorAnalysis,
    ProgressEvaluation,
}

/// Advanced reasoning engine that uses LLM capabilities for sophisticated analysis
pub struct LLMReasoningEngine {
    engine: Arc<Box<dyn Engine>>,
    reasoning_prompts: ReasoningPrompts,
    capabilities: Vec<ReasoningCapability>,
}

/// Collection of prompts for different reasoning tasks
#[derive(Debug, Clone)]
pub struct ReasoningPrompts {
    pub goal_analysis: String,
    pub task_decomposition: String,
    pub action_planning: String,
    pub context_analysis: String,
    pub problem_solving: String,
    pub self_reflection: String,
    pub strategy_adjustment: String,
}

impl LLMReasoningEngine {
    /// Create a new LLM-based reasoning engine
    pub fn new(engine: Arc<Box<dyn Engine>>) -> Self {
        Self {
            engine,
            reasoning_prompts: ReasoningPrompts::default(),
            capabilities: vec![
                ReasoningCapability::GoalDecomposition,
                ReasoningCapability::TaskPlanning,
                ReasoningCapability::ProblemSolving,
                ReasoningCapability::ContextAnalysis,
                ReasoningCapability::StrategyFormulation,
                ReasoningCapability::SelfReflection,
                ReasoningCapability::ErrorAnalysis,
                ReasoningCapability::ProgressEvaluation,
            ],
        }
    }

    /// Create reasoning engine with custom prompts
    pub fn with_prompts(engine: Arc<Box<dyn Engine>>, prompts: ReasoningPrompts) -> Self {
        Self {
            engine,
            reasoning_prompts: prompts,
            capabilities: vec![
                ReasoningCapability::GoalDecomposition,
                ReasoningCapability::TaskPlanning,
                ReasoningCapability::ProblemSolving,
                ReasoningCapability::ContextAnalysis,
                ReasoningCapability::StrategyFormulation,
                ReasoningCapability::SelfReflection,
                ReasoningCapability::ErrorAnalysis,
                ReasoningCapability::ProgressEvaluation,
            ],
        }
    }

    /// Perform goal analysis reasoning
    async fn analyze_goal(&self, context: &ExecutionContext) -> Result<ReasoningResult> {
        let prompt = self.build_goal_analysis_prompt(context);
        let response = self.execute_reasoning(&prompt).await?;
        
        Ok(ReasoningResult {
            reasoning_type: ReasoningType::GoalAnalysis,
            input_context: format!("Goal: {:?}", context.get_current_goal()),
            reasoning_output: response.clone(),
            confidence_score: self.extract_confidence(&response),
            goal_achieved_confidence: self.extract_goal_achievement_confidence(&response),
            next_actions: self.extract_next_actions(&response),
        })
    }

    /// Perform task decomposition reasoning
    async fn decompose_task(&self, context: &ExecutionContext) -> Result<ReasoningResult> {
        let prompt = self.build_task_decomposition_prompt(context);
        let response = self.execute_reasoning(&prompt).await?;
        
        Ok(ReasoningResult {
            reasoning_type: ReasoningType::TaskDecomposition,
            input_context: format!("Current task: {:?}", context.get_current_task()),
            reasoning_output: response.clone(),
            confidence_score: self.extract_confidence(&response),
            goal_achieved_confidence: 0.0, // Not applicable for task decomposition
            next_actions: self.extract_next_actions(&response),
        })
    }

    /// Perform action planning reasoning
    async fn plan_action(&self, context: &ExecutionContext) -> Result<ReasoningResult> {
        let prompt = self.build_action_planning_prompt(context);
        let response = self.execute_reasoning(&prompt).await?;
        
        Ok(ReasoningResult {
            reasoning_type: ReasoningType::ActionPlanning,
            input_context: format!("Context: {:?}", context.get_summary()),
            reasoning_output: response.clone(),
            confidence_score: self.extract_confidence(&response),
            goal_achieved_confidence: 0.0, // Not applicable for action planning
            next_actions: self.extract_next_actions(&response),
        })
    }

    /// Perform context analysis reasoning
    async fn analyze_context(&self, context: &ExecutionContext) -> Result<ReasoningResult> {
        let prompt = self.build_context_analysis_prompt(context);
        let response = self.execute_reasoning(&prompt).await?;
        
        Ok(ReasoningResult {
            reasoning_type: ReasoningType::ContextAnalysis,
            input_context: format!("Full context: {:?}", context),
            reasoning_output: response.clone(),
            confidence_score: self.extract_confidence(&response),
            goal_achieved_confidence: self.extract_goal_achievement_confidence(&response),
            next_actions: self.extract_next_actions(&response),
        })
    }

    /// Perform self-reflection reasoning
    async fn self_reflect(&self, context: &ExecutionContext) -> Result<ReasoningResult> {
        let prompt = self.build_self_reflection_prompt(context);
        let response = self.execute_reasoning(&prompt).await?;
        
        Ok(ReasoningResult {
            reasoning_type: ReasoningType::SelfReflection,
            input_context: format!("Progress: {:?}", context.get_progress_summary()),
            reasoning_output: response.clone(),
            confidence_score: self.extract_confidence(&response),
            goal_achieved_confidence: self.extract_goal_achievement_confidence(&response),
            next_actions: self.extract_next_actions(&response),
        })
    }

    /// Execute reasoning with the LLM engine
    async fn execute_reasoning(&self, prompt: &str) -> Result<String> {
        let request = Request {
            flowname: "reasoning".to_string(),
            payload: prompt.to_string(),
        };

        let response = Pin::from(self.engine.execute(&request)).await?;
        Ok(response.content)
    }

    /// Build goal analysis prompt
    fn build_goal_analysis_prompt(&self, context: &ExecutionContext) -> String {
        format!(
            "{}\n\nCurrent Goal: {:?}\nContext: {:?}\n\nPlease analyze the goal and provide:\n1. Goal clarity assessment\n2. Feasibility analysis\n3. Required resources\n4. Success criteria\n5. Potential obstacles\n6. Confidence score (0.0-1.0)\n7. Next recommended actions",
            self.reasoning_prompts.goal_analysis,
            context.get_current_goal(),
            context.get_summary()
        )
    }

    /// Build task decomposition prompt
    fn build_task_decomposition_prompt(&self, context: &ExecutionContext) -> String {
        format!(
            "{}\n\nCurrent Task: {:?}\nGoal: {:?}\nContext: {:?}\n\nPlease decompose the task into:\n1. Subtasks with clear objectives\n2. Dependencies between subtasks\n3. Priority ordering\n4. Resource requirements\n5. Success criteria for each subtask\n6. Confidence score (0.0-1.0)\n7. Next immediate actions",
            self.reasoning_prompts.task_decomposition,
            context.get_current_task(),
            context.get_current_goal(),
            context.get_summary()
        )
    }

    /// Build action planning prompt
    fn build_action_planning_prompt(&self, context: &ExecutionContext) -> String {
        format!(
            "{}\n\nCurrent Situation: {:?}\nAvailable Tools: {:?}\nRecent Actions: {:?}\n\nPlease plan the next action by:\n1. Analyzing current situation\n2. Identifying best next action\n3. Specifying action parameters\n4. Predicting expected outcomes\n5. Identifying potential risks\n6. Confidence score (0.0-1.0)\n7. Alternative actions if primary fails",
            self.reasoning_prompts.action_planning,
            context.get_summary(),
            context.get_available_tools(),
            context.get_recent_actions()
        )
    }

    /// Build context analysis prompt
    fn build_context_analysis_prompt(&self, context: &ExecutionContext) -> String {
        format!(
            "{}\n\nFull Context: {:?}\n\nPlease analyze the context by:\n1. Identifying key information\n2. Assessing current progress\n3. Detecting patterns or trends\n4. Identifying missing information\n5. Evaluating context quality\n6. Goal achievement likelihood\n7. Recommended focus areas",
            self.reasoning_prompts.context_analysis,
            context
        )
    }

    /// Build self-reflection prompt
    fn build_self_reflection_prompt(&self, context: &ExecutionContext) -> String {
        format!(
            "{}\n\nProgress Summary: {:?}\nActions Taken: {:?}\nResults Achieved: {:?}\n\nPlease reflect on:\n1. What has worked well\n2. What hasn't worked\n3. Lessons learned\n4. Strategy adjustments needed\n5. Confidence in current approach\n6. Goal achievement probability\n7. Recommended strategy changes",
            self.reasoning_prompts.self_reflection,
            context.get_progress_summary(),
            context.get_action_history(),
            context.get_results_summary()
        )
    }

    /// Extract confidence score from reasoning response
    fn extract_confidence(&self, response: &str) -> f64 {
        // Simple regex-based extraction - could be enhanced with more sophisticated parsing
        let patterns = vec![
            r"confidence[:\s]*([0-9]*\.?[0-9]+)",  // "confidence: 0.85" or "confidence 0.85"
            r"confidence\s+(?:score\s+)?(?:is\s+)?([0-9]*\.?[0-9]+)",  // "confidence score is 0.85"
        ];

        for pattern in patterns {
            if let Ok(regex) = regex::Regex::new(pattern) {
                if let Some(captures) = regex.captures(&response.to_lowercase()) {
                    if let Some(score_str) = captures.get(1) {
                        return score_str.as_str().parse().unwrap_or(0.5);
                    }
                }
            }
        }
        0.5_f64 // Default confidence if not found
    }

    /// Extract goal achievement confidence from reasoning response
    fn extract_goal_achievement_confidence(&self, response: &str) -> f64 {
        // Look for goal achievement indicators
        if let Ok(regex) = regex::Regex::new(r"goal.*achievement.*([0-9]*\.?[0-9]+)") {
            if let Some(captures) = regex.captures(&response.to_lowercase()) {
                if let Some(score_str) = captures.get(1) {
                    return score_str.as_str().parse().unwrap_or(0.0);
                }
            }
        }
        
        // Check for completion indicators
        let completion_indicators = ["completed", "achieved", "finished", "done", "success"];
        let completion_count = completion_indicators
            .iter()
            .filter(|&indicator| response.to_lowercase().contains(indicator))
            .count();
        
        if completion_count > 0 {
            0.8_f64 // High confidence if completion indicators found
        } else {
            0.0_f64 // Low confidence otherwise
        }
    }

    /// Extract next actions from reasoning response
    fn extract_next_actions(&self, response: &str) -> Vec<String> {
        let mut actions = Vec::new();
        
        // Look for numbered lists or bullet points
        for line in response.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("1.") || trimmed.starts_with("2.") || 
               trimmed.starts_with("3.") || trimmed.starts_with("-") || 
               trimmed.starts_with("*") {
                if let Some(action) = trimmed.split_once('.').or_else(|| trimmed.split_once(' ')) {
                    actions.push(action.1.trim().to_string());
                }
            }
        }
        
        // If no structured actions found, return the whole response as a single action
        if actions.is_empty() {
            actions.push(response.to_string());
        }
        
        actions
    }
}

#[async_trait]
impl ReasoningEngine for LLMReasoningEngine {
    async fn reason(&self, context: &ExecutionContext) -> Result<ReasoningResult> {
        // Determine the most appropriate reasoning type based on context
        let reasoning_type = self.determine_reasoning_type(context);
        
        match reasoning_type {
            ReasoningType::GoalAnalysis => self.analyze_goal(context).await,
            ReasoningType::TaskDecomposition => self.decompose_task(context).await,
            ReasoningType::ActionPlanning => self.plan_action(context).await,
            ReasoningType::ContextAnalysis => self.analyze_context(context).await,
            ReasoningType::SelfReflection => self.self_reflect(context).await,
            _ => self.analyze_context(context).await, // Default to context analysis
        }
    }

    fn get_capabilities(&self) -> Vec<ReasoningCapability> {
        self.capabilities.clone()
    }

    fn can_handle(&self, reasoning_type: &ReasoningType) -> bool {
        match reasoning_type {
            ReasoningType::GoalAnalysis => true,
            ReasoningType::TaskDecomposition => true,
            ReasoningType::ActionPlanning => true,
            ReasoningType::ContextAnalysis => true,
            ReasoningType::ProblemSolving => true,
            ReasoningType::SelfReflection => true,
            ReasoningType::StrategyAdjustment => true,
        }
    }
}

impl LLMReasoningEngine {
    /// Determine the most appropriate reasoning type for the current context
    fn determine_reasoning_type(&self, context: &ExecutionContext) -> ReasoningType {
        // Simple heuristics - could be enhanced with more sophisticated logic
        if context.is_goal_unclear() {
            ReasoningType::GoalAnalysis
        } else if context.needs_task_decomposition() {
            ReasoningType::TaskDecomposition
        } else if context.needs_action_planning() {
            ReasoningType::ActionPlanning
        } else if context.iteration_count() % 5 == 0 {
            ReasoningType::SelfReflection
        } else {
            ReasoningType::ContextAnalysis
        }
    }
}

impl Default for ReasoningPrompts {
    fn default() -> Self {
        Self {
            goal_analysis: "You are an expert goal analyst. Analyze the given goal for clarity, feasibility, and actionability.".to_string(),
            task_decomposition: "You are an expert task planner. Break down complex tasks into manageable subtasks with clear dependencies.".to_string(),
            action_planning: "You are an expert action planner. Determine the best next action based on current context and available tools.".to_string(),
            context_analysis: "You are an expert context analyzer. Analyze the current situation and identify key insights and patterns.".to_string(),
            problem_solving: "You are an expert problem solver. Identify problems and generate creative solutions.".to_string(),
            self_reflection: "You are an expert at self-reflection. Analyze progress, identify lessons learned, and suggest improvements.".to_string(),
            strategy_adjustment: "You are an expert strategist. Evaluate current strategy and recommend adjustments for better outcomes.".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reasoning_prompts_default() {
        let prompts = ReasoningPrompts::default();
        assert!(!prompts.goal_analysis.is_empty());
        assert!(!prompts.task_decomposition.is_empty());
        assert!(!prompts.action_planning.is_empty());
    }

    #[test]
    fn test_extract_confidence() {
        let engine = LLMReasoningEngine::new(Arc::new(Box::new(MockEngine)));
        
        let response1 = "The confidence score is 0.85 for this analysis.";
        assert_eq!(engine.extract_confidence(response1), 0.85);
        
        let response2 = "Confidence: 0.7";
        assert_eq!(engine.extract_confidence(response2), 0.7);
        
        let response3 = "No confidence mentioned";
        assert_eq!(engine.extract_confidence(response3), 0.5);
    }

    #[test]
    fn test_extract_next_actions() {
        let engine = LLMReasoningEngine::new(Arc::new(Box::new(MockEngine)));
        
        let response = "Next actions:\n1. Analyze the code\n2. Write tests\n3. Deploy changes";
        let actions = engine.extract_next_actions(response);
        assert_eq!(actions.len(), 3);
        assert!(actions[0].contains("Analyze the code"));
    }
}

// Mock engine for testing
#[allow(dead_code)]
struct MockEngine;

#[async_trait]
impl Engine for MockEngine {
    fn execute<'a>(&'a self, _request: &'a Request) -> Box<dyn std::future::Future<Output = Result<fluent_core::types::Response>> + Send + 'a> {
        Box::new(async move {
            Ok(fluent_core::types::Response {
                content: "Mock response".to_string(),
                usage: Usage {
                    prompt_tokens: 10,
                    completion_tokens: 20,
                    total_tokens: 30,
                },
                model: "mock-model".to_string(),
                finish_reason: Some("stop".to_string()),
                cost: Cost {
                    prompt_cost: 0.001,
                    completion_cost: 0.002,
                    total_cost: 0.003,
                },
            })
        })
    }

    fn upsert<'a>(&'a self, _request: &'a fluent_core::types::UpsertRequest) -> Box<dyn std::future::Future<Output = Result<fluent_core::types::UpsertResponse>> + Send + 'a> {
        Box::new(async move {
            Ok(fluent_core::types::UpsertResponse {
                processed_files: vec!["mock_file.txt".to_string()],
                errors: Vec::new(),
            })
        })
    }

    fn get_neo4j_client(&self) -> Option<&std::sync::Arc<Neo4jClient>> {
        None
    }

    fn get_session_id(&self) -> Option<String> {
        None
    }

    fn extract_content(&self, _value: &serde_json::Value) -> Option<fluent_core::types::ExtractedContent> {
        None
    }

    fn upload_file<'a>(&'a self, _file_path: &'a std::path::Path) -> Box<dyn std::future::Future<Output = Result<String>> + Send + 'a> {
        Box::new(async move {
            Ok("Mock upload".to_string())
        })
    }

    fn process_request_with_file<'a>(&'a self, _request: &'a Request, _file_path: &'a std::path::Path) -> Box<dyn std::future::Future<Output = Result<fluent_core::types::Response>> + Send + 'a> {
        Box::new(async move {
            Ok(fluent_core::types::Response {
                content: "Mock response with file".to_string(),
                usage: Usage {
                    prompt_tokens: 15,
                    completion_tokens: 25,
                    total_tokens: 40,
                },
                model: "mock-model-file".to_string(),
                finish_reason: Some("stop".to_string()),
                cost: Cost {
                    prompt_cost: 0.0015,
                    completion_cost: 0.0025,
                    total_cost: 0.004,
                },
            })
        })
    }
}
