//! Advanced reasoning engines for sophisticated problem solving
//!
//! This module contains various reasoning engines that implement different
//! cognitive patterns for autonomous problem solving, including multi-modal
//! reasoning capabilities for processing text, code, images, and audio.

pub mod algorithmic_patterns;
pub mod chain_of_thought;
pub mod enhanced_multi_modal;
pub mod meta_reasoning;
pub mod multi_modal;
pub mod sysadmin_patterns;
pub mod tree_of_thought;

pub use algorithmic_patterns::{
    AlgorithmCategory, AlgorithmGuidance, AlgorithmPattern, AlgorithmPatternDetector,
    PatternDetectionResult,
};
pub use sysadmin_patterns::{
    SysadminCategory, SysadminDetectionResult, SysadminGuidance, SysadminPattern,
    SysadminPatternDetector,
};
pub use chain_of_thought::{ChainOfThoughtEngine, CoTConfig, CoTReasoningResult};
pub use enhanced_multi_modal::{
    EnhancedMultiModalEngine, EnhancedReasoningConfig, EnhancedReasoningResult,
};
pub use meta_reasoning::{MetaConfig, MetaReasoningEngine, MetaReasoningResult};
pub use multi_modal::{
    AudioData, BinaryData, CodeContent, CrossModalRelationship, ImageData, MultiModalInput,
    MultiModalReasoningEngine, MultiModalReasoningResult, StructuredData,
};
pub use tree_of_thought::{ToTConfig, ToTReasoningResult, TreeOfThoughtEngine};

// Re-export the main reasoning traits
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::context::ExecutionContext;

/// Trait for reasoning engines that can analyze context and plan actions
#[async_trait]
pub trait ReasoningEngine: Send + Sync {
    /// Analyze the current execution context and generate reasoning output
    async fn reason(&self, prompt: &str, context: &ExecutionContext) -> Result<String>;

    /// Get the reasoning capabilities of this engine
    async fn get_capabilities(&self) -> Vec<ReasoningCapability>;

    /// Get the current confidence level of this engine
    async fn get_confidence(&self) -> f64;
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
    PerformanceEvaluation,
    MultiPathExploration,
    QualityEvaluation,
    BacktrackingSearch,
    ConfidenceScoring,
    ChainOfThought,
    MetaCognition,
    AnalogicalReasoning,
    CausalReasoning,
    AlgorithmicReasoning,
    SysadminReasoning,
}

/// Structured output from a reasoning step with validated schema
///
/// This replaces ad-hoc string parsing with a well-defined schema that
/// can be validated and used programmatically.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredReasoningOutput {
    /// High-level summary of the reasoning (1-2 sentences)
    pub summary: String,

    /// Detailed reasoning chain (thought process)
    pub reasoning_chain: Vec<ReasoningThought>,

    /// Assessment of progress toward the goal
    pub goal_assessment: GoalAssessment,

    /// Proposed next actions to take
    pub proposed_actions: Vec<ProposedAction>,

    /// Self-assessment confidence (0.0-1.0)
    pub confidence: f64,

    /// Any issues or blockers identified
    pub blockers: Vec<String>,

    /// Metadata for debugging and analysis
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

/// A single thought in the reasoning chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningThought {
    /// Type of reasoning step
    pub thought_type: ThoughtType,
    /// Content of the thought
    pub content: String,
    /// Confidence in this specific thought (0.0-1.0)
    pub confidence: f64,
}

/// Types of thoughts in a reasoning chain
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ThoughtType {
    /// Analyzing the current situation
    Analysis,
    /// Making a decision
    Decision,
    /// Considering alternatives
    Consideration,
    /// Concluding based on evidence
    Conclusion,
    /// Identifying a problem
    Problem,
    /// Proposing a solution
    Solution,
}

/// Assessment of progress toward the goal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalAssessment {
    /// Estimated progress percentage (0.0-1.0)
    pub progress_percentage: f64,
    /// Whether the goal is believed to be achieved
    pub is_achieved: bool,
    /// Confidence in the achievement assessment (0.0-1.0)
    pub achievement_confidence: f64,
    /// Evidence supporting the assessment
    pub evidence: Vec<String>,
    /// Remaining steps if not achieved
    pub remaining_steps: Vec<String>,
}

/// A proposed action to take
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedAction {
    /// Type of action to take
    pub action_type: ProposedActionType,
    /// Description of what to do
    pub description: String,
    /// Priority (higher = more important)
    pub priority: u8,
    /// Expected outcome
    pub expected_outcome: Option<String>,
}

/// Types of actions the agent can propose
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProposedActionType {
    /// Execute a tool
    ExecuteTool,
    /// Write code
    WriteCode,
    /// Read a file
    ReadFile,
    /// Execute a command
    ExecuteCommand,
    /// Search for information
    Search,
    /// Ask for clarification
    AskClarification,
    /// Report completion
    ReportComplete,
    /// Other action
    Other,
}

impl Default for StructuredReasoningOutput {
    fn default() -> Self {
        Self {
            summary: String::new(),
            reasoning_chain: Vec::new(),
            goal_assessment: GoalAssessment {
                progress_percentage: 0.0,
                is_achieved: false,
                achievement_confidence: 0.0,
                evidence: Vec::new(),
                remaining_steps: Vec::new(),
            },
            proposed_actions: Vec::new(),
            confidence: 0.0,
            blockers: Vec::new(),
            metadata: std::collections::HashMap::new(),
        }
    }
}

impl StructuredReasoningOutput {
    /// Parse a raw reasoning string into structured output
    ///
    /// This attempts to extract structure from unstructured LLM output
    /// using heuristics and pattern matching.
    pub fn from_raw_output(raw: &str) -> Self {
        let mut output = Self::default();
        output.summary = Self::extract_summary(raw);
        output.reasoning_chain = Self::extract_reasoning_chain(raw);
        output.goal_assessment = Self::extract_goal_assessment(raw);
        output.proposed_actions = Self::extract_proposed_actions(raw);
        output.confidence = Self::estimate_confidence(raw);
        output.blockers = Self::extract_blockers(raw);
        output
    }

    /// Validate the structured output
    pub fn validate(&self) -> Result<()> {
        if self.summary.is_empty() && self.reasoning_chain.is_empty() {
            return Err(anyhow::anyhow!("Reasoning output is empty"));
        }
        if !(0.0..=1.0).contains(&self.confidence) {
            return Err(anyhow::anyhow!(
                "Confidence must be between 0.0 and 1.0, got {}",
                self.confidence
            ));
        }
        if !(0.0..=1.0).contains(&self.goal_assessment.progress_percentage) {
            return Err(anyhow::anyhow!(
                "Progress percentage must be between 0.0 and 1.0"
            ));
        }
        Ok(())
    }

    /// Extract a summary from raw output
    fn extract_summary(raw: &str) -> String {
        // Look for explicit summary markers
        let lines: Vec<&str> = raw.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            let lower = line.to_lowercase();
            if lower.starts_with("summary:")
                || lower.starts_with("**summary**")
                || lower.starts_with("# summary")
            {
                // Return the content after the marker
                let content = line.split(':').nth(1).map(|s| s.trim()).unwrap_or("");
                if !content.is_empty() {
                    return content.to_string();
                }
                // Otherwise return next line
                if i + 1 < lines.len() {
                    return lines[i + 1].trim().to_string();
                }
            }
        }

        // Fall back to first non-empty line
        lines
            .iter()
            .find(|l| !l.trim().is_empty())
            .map(|l| l.trim().to_string())
            .unwrap_or_default()
    }

    /// Extract the reasoning chain from raw output
    fn extract_reasoning_chain(raw: &str) -> Vec<ReasoningThought> {
        let mut thoughts = Vec::new();
        let lines: Vec<&str> = raw.lines().collect();

        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Skip headers and markers
            if trimmed.starts_with('#') || trimmed.starts_with("**") {
                continue;
            }

            // Look for numbered steps or bullet points
            let is_list_item = trimmed.starts_with('-')
                || trimmed.starts_with('*')
                || trimmed.starts_with("•")
                || trimmed.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false);

            if is_list_item {
                let content = trimmed
                    .trim_start_matches(|c: char| c == '-' || c == '*' || c == '•' || c.is_ascii_digit() || c == '.' || c == ')')
                    .trim()
                    .to_string();

                if content.is_empty() {
                    continue;
                }

                let thought_type = Self::classify_thought(&content);
                thoughts.push(ReasoningThought {
                    thought_type,
                    content,
                    confidence: 0.7, // Default confidence for extracted thoughts
                });
            }
        }

        thoughts
    }

    /// Classify a thought based on its content
    fn classify_thought(content: &str) -> ThoughtType {
        let lower = content.to_lowercase();

        if lower.contains("problem") || lower.contains("issue") || lower.contains("error") {
            ThoughtType::Problem
        } else if lower.contains("solution") || lower.contains("fix") || lower.contains("resolve") {
            ThoughtType::Solution
        } else if lower.contains("decide") || lower.contains("will") || lower.contains("should") {
            ThoughtType::Decision
        } else if lower.contains("consider") || lower.contains("alternative") || lower.contains("option") {
            ThoughtType::Consideration
        } else if lower.contains("therefore") || lower.contains("conclude") || lower.contains("result") {
            ThoughtType::Conclusion
        } else {
            ThoughtType::Analysis
        }
    }

    /// Extract goal assessment from raw output
    fn extract_goal_assessment(raw: &str) -> GoalAssessment {
        let lower = raw.to_lowercase();

        // Check for achievement indicators
        let is_achieved = lower.contains("goal achieved")
            || lower.contains("task complete")
            || lower.contains("successfully completed")
            || lower.contains("finished implementing")
            || (lower.contains("complete") && lower.contains("success"));

        // Estimate progress based on keywords
        let progress = if is_achieved {
            1.0
        } else if lower.contains("almost") || lower.contains("nearly") {
            0.8
        } else if lower.contains("halfway") || lower.contains("50%") {
            0.5
        } else if lower.contains("started") || lower.contains("beginning") {
            0.2
        } else {
            0.3 // Default progress
        };

        // Achievement confidence based on strength of language
        let achievement_confidence = if is_achieved {
            if lower.contains("definitely") || lower.contains("certainly") {
                0.95
            } else if lower.contains("believe") || lower.contains("think") {
                0.7
            } else {
                0.85
            }
        } else {
            0.3
        };

        GoalAssessment {
            progress_percentage: progress,
            is_achieved,
            achievement_confidence,
            evidence: Vec::new(), // Would need more sophisticated extraction
            remaining_steps: Vec::new(),
        }
    }

    /// Extract proposed actions from raw output
    fn extract_proposed_actions(raw: &str) -> Vec<ProposedAction> {
        let mut actions = Vec::new();
        let lower = raw.to_lowercase();

        // Common action patterns
        let action_keywords = [
            ("write", ProposedActionType::WriteCode),
            ("create", ProposedActionType::WriteCode),
            ("implement", ProposedActionType::WriteCode),
            ("read", ProposedActionType::ReadFile),
            ("open", ProposedActionType::ReadFile),
            ("execute", ProposedActionType::ExecuteCommand),
            ("run", ProposedActionType::ExecuteCommand),
            ("search", ProposedActionType::Search),
            ("find", ProposedActionType::Search),
            ("ask", ProposedActionType::AskClarification),
            ("clarify", ProposedActionType::AskClarification),
        ];

        for (keyword, action_type) in &action_keywords {
            if lower.contains(keyword) {
                // Find the sentence containing this keyword
                for line in raw.lines() {
                    if line.to_lowercase().contains(keyword) {
                        actions.push(ProposedAction {
                            action_type: action_type.clone(),
                            description: line.trim().to_string(),
                            priority: 5,
                            expected_outcome: None,
                        });
                        break;
                    }
                }
            }
        }

        // Deduplicate by description
        actions.sort_by(|a, b| a.description.cmp(&b.description));
        actions.dedup_by(|a, b| a.description == b.description);

        actions
    }

    /// Estimate confidence from raw output
    fn estimate_confidence(raw: &str) -> f64 {
        let lower = raw.to_lowercase();

        // High confidence indicators
        if lower.contains("definitely")
            || lower.contains("certainly")
            || lower.contains("confident")
        {
            return 0.9;
        }

        // Medium-high confidence
        if lower.contains("likely") || lower.contains("probably") {
            return 0.75;
        }

        // Low confidence indicators
        if lower.contains("uncertain")
            || lower.contains("unclear")
            || lower.contains("not sure")
            || lower.contains("maybe")
        {
            return 0.4;
        }

        // Error/problem indicators reduce confidence
        if lower.contains("error") || lower.contains("failed") || lower.contains("problem") {
            return 0.5;
        }

        0.7 // Default confidence
    }

    /// Extract blockers from raw output
    fn extract_blockers(raw: &str) -> Vec<String> {
        let mut blockers = Vec::new();
        let lower = raw.to_lowercase();

        // Common blocker patterns
        let blocker_keywords = ["blocked by", "cannot", "unable to", "need to", "waiting for", "requires"];

        for line in raw.lines() {
            let line_lower = line.to_lowercase();
            for keyword in &blocker_keywords {
                if line_lower.contains(keyword) {
                    blockers.push(line.trim().to_string());
                    break;
                }
            }
        }

        blockers
    }
}

/// Composite reasoning engine that combines multiple reasoning approaches
pub struct CompositeReasoningEngine {
    engines: Vec<Box<dyn ReasoningEngine>>,
    selection_strategy: ReasoningSelectionStrategy,
}

/// Strategy for selecting which reasoning engine to use
#[derive(Debug, Clone)]
pub enum ReasoningSelectionStrategy {
    /// Use the engine with highest confidence
    HighestConfidence,
    /// Use Tree-of-Thought for complex problems, others for simple ones
    ComplexityBased,
    /// Use all engines and combine results
    EnsembleVoting,
    /// Use specific engine for specific problem types
    ProblemTypeMatching,
}

impl CompositeReasoningEngine {
    /// Create a new composite reasoning engine
    pub fn new(
        engines: Vec<Box<dyn ReasoningEngine>>,
        strategy: ReasoningSelectionStrategy,
    ) -> Self {
        Self {
            engines,
            selection_strategy: strategy,
        }
    }

    /// Add a reasoning engine to the composite
    pub fn add_engine(&mut self, engine: Box<dyn ReasoningEngine>) {
        self.engines.push(engine);
    }

    /// Determine complexity of a problem
    fn assess_problem_complexity(&self, prompt: &str) -> f64 {
        let complexity_indicators = [
            ("multiple", 0.2),
            ("complex", 0.3),
            ("analyze", 0.1),
            ("compare", 0.2),
            ("evaluate", 0.2),
            ("synthesize", 0.3),
            ("optimize", 0.3),
            ("design", 0.2),
            ("create", 0.1),
            ("implement", 0.2),
        ];

        let mut score = 0.0;
        let prompt_lower = prompt.to_lowercase();

        for (indicator, weight) in complexity_indicators {
            if prompt_lower.contains(indicator) {
                score += weight;
            }
        }

        // Length-based complexity
        score += (prompt.len() as f64 / 1000.0).min(0.3);

        score.min(1.0)
    }
}

#[async_trait]
impl ReasoningEngine for CompositeReasoningEngine {
    async fn reason(&self, prompt: &str, context: &ExecutionContext) -> Result<String> {
        match self.selection_strategy {
            ReasoningSelectionStrategy::ComplexityBased => {
                let complexity = self.assess_problem_complexity(prompt);

                if complexity > 0.5 {
                    // Use Tree-of-Thought for complex problems
                    if let Some(tot_engine) = self.engines.iter().find(|_e| {
                        // Check if this is a Tree-of-Thought engine by type name
                        std::any::type_name::<dyn ReasoningEngine>().contains("TreeOfThought")
                    }) {
                        return tot_engine.reason(prompt, context).await;
                    }
                }

                // Fallback to first available engine
                if let Some(engine) = self.engines.first() {
                    engine.reason(prompt, context).await
                } else {
                    Ok("No reasoning engines available".to_string())
                }
            }

            ReasoningSelectionStrategy::HighestConfidence => {
                let mut best_engine: Option<&Box<dyn ReasoningEngine>> = None;
                let mut best_confidence = 0.0;

                for engine in &self.engines {
                    let confidence = engine.get_confidence().await;
                    if confidence > best_confidence {
                        best_confidence = confidence;
                        best_engine = Some(engine);
                    }
                }

                if let Some(engine) = best_engine {
                    engine.reason(prompt, context).await
                } else {
                    Ok("No reasoning engines available".to_string())
                }
            }

            ReasoningSelectionStrategy::EnsembleVoting => {
                let mut results = Vec::new();

                for engine in &self.engines {
                    if let Ok(result) = engine.reason(prompt, context).await {
                        results.push(result);
                    }
                }

                if results.is_empty() {
                    Ok("No reasoning engines produced results".to_string())
                } else {
                    Ok(format!(
                        "Ensemble Reasoning Results:\n\n{}",
                        results
                            .into_iter()
                            .enumerate()
                            .map(|(i, r)| format!("Engine {}: {}", i + 1, r))
                            .collect::<Vec<_>>()
                            .join("\n\n---\n\n")
                    ))
                }
            }

            ReasoningSelectionStrategy::ProblemTypeMatching => {
                // Simple heuristic-based engine selection
                let prompt_lower = prompt.to_lowercase();

                if prompt_lower.contains("explore") || prompt_lower.contains("alternative") {
                    // Use Tree-of-Thought for exploration
                    if let Some(tot_engine) = self.engines.first() {
                        return tot_engine.reason(prompt, context).await;
                    }
                }

                // Default to first engine
                if let Some(engine) = self.engines.first() {
                    engine.reason(prompt, context).await
                } else {
                    Ok("No reasoning engines available".to_string())
                }
            }
        }
    }

    async fn get_capabilities(&self) -> Vec<ReasoningCapability> {
        let mut all_capabilities = Vec::new();

        for engine in &self.engines {
            let mut capabilities = engine.get_capabilities().await;
            all_capabilities.append(&mut capabilities);
        }

        // Deduplicate capabilities
        all_capabilities.sort_by_key(|c| format!("{:?}", c));
        all_capabilities.dedup_by_key(|c| format!("{:?}", c));

        all_capabilities
    }

    async fn get_confidence(&self) -> f64 {
        if self.engines.is_empty() {
            return 0.0;
        }

        let mut total_confidence = 0.0;
        let mut count = 0;

        for engine in &self.engines {
            total_confidence += engine.get_confidence().await;
            count += 1;
        }

        if count > 0 {
            total_confidence / count as f64
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_structured_output_from_raw_basic() {
        let raw = r#"Summary: Analyzing the file structure

- First, I need to read the main.rs file
- Then I will implement the changes
- Finally, run the tests to verify

The goal is almost complete.
"#;
        let output = StructuredReasoningOutput::from_raw_output(raw);

        assert!(!output.summary.is_empty());
        assert!(!output.reasoning_chain.is_empty());
        assert!(output.goal_assessment.progress_percentage > 0.5);
    }

    #[test]
    fn test_structured_output_goal_achieved() {
        let raw = "The task has been successfully completed. Goal achieved!";
        let output = StructuredReasoningOutput::from_raw_output(raw);

        assert!(output.goal_assessment.is_achieved);
        assert!(output.goal_assessment.achievement_confidence > 0.8);
        assert_eq!(output.goal_assessment.progress_percentage, 1.0);
    }

    #[test]
    fn test_structured_output_goal_not_achieved() {
        let raw = "I'm starting to work on this task. Let me begin by analyzing the requirements.";
        let output = StructuredReasoningOutput::from_raw_output(raw);

        assert!(!output.goal_assessment.is_achieved);
        assert!(output.goal_assessment.progress_percentage < 0.5);
    }

    #[test]
    fn test_structured_output_extracts_actions() {
        let raw = r#"
I will write a new file called main.rs
Then I need to read the existing config
Finally, execute the tests
"#;
        let output = StructuredReasoningOutput::from_raw_output(raw);

        assert!(!output.proposed_actions.is_empty());
        // Check that action types are correctly identified
        let action_types: Vec<_> = output.proposed_actions.iter().map(|a| &a.action_type).collect();
        assert!(action_types.contains(&&ProposedActionType::WriteCode));
        assert!(action_types.contains(&&ProposedActionType::ReadFile));
    }

    #[test]
    fn test_structured_output_extracts_blockers() {
        let raw = "I cannot proceed because the API key is missing. Need to wait for user input.";
        let output = StructuredReasoningOutput::from_raw_output(raw);

        assert!(!output.blockers.is_empty());
    }

    #[test]
    fn test_structured_output_confidence_high() {
        let raw = "I am definitely confident this approach will work.";
        let output = StructuredReasoningOutput::from_raw_output(raw);

        assert!(output.confidence >= 0.9);
    }

    #[test]
    fn test_structured_output_confidence_low() {
        let raw = "I'm uncertain about this approach and not sure if it will work.";
        let output = StructuredReasoningOutput::from_raw_output(raw);

        assert!(output.confidence < 0.5);
    }

    #[test]
    fn test_structured_output_validation_passes() {
        let raw = "Summary: Valid output with content\n- Step one\n- Step two";
        let output = StructuredReasoningOutput::from_raw_output(raw);

        assert!(output.validate().is_ok());
    }

    #[test]
    fn test_thought_type_classification() {
        assert_eq!(
            StructuredReasoningOutput::classify_thought("There is a problem with the API"),
            ThoughtType::Problem
        );
        assert_eq!(
            StructuredReasoningOutput::classify_thought("The solution is to add retries"),
            ThoughtType::Solution
        );
        assert_eq!(
            StructuredReasoningOutput::classify_thought("I will implement this feature"),
            ThoughtType::Decision
        );
        assert_eq!(
            StructuredReasoningOutput::classify_thought("Let me consider alternative approaches"),
            ThoughtType::Consideration
        );
    }
}
