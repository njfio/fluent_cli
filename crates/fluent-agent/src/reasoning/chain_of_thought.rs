//! Chain-of-Thought Reasoning Engine
//!
//! This module implements Chain-of-Thought (CoT) reasoning with verification
//! and backtracking capabilities. Unlike Tree-of-Thought which explores multiple
//! paths simultaneously, CoT follows a single reasoning chain but can backtrack
//! and explore alternatives when verification fails.
//!
//! The engine maintains a linear chain of reasoning steps, validates each step,
//! and can backtrack to previous steps if inconsistencies or errors are detected.

use anyhow::Result;
use serde::{Deserialize, Serialize};

use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::reasoning::{ReasoningEngine, ReasoningCapability};
use crate::context::ExecutionContext;
use fluent_core::traits::Engine;

/// Chain-of-Thought reasoning engine with verification and backtracking
pub struct ChainOfThoughtEngine {
    base_engine: Arc<dyn Engine>,
    config: CoTConfig,
    reasoning_chain: Arc<RwLock<ReasoningChain>>,
    verification_engine: Arc<RwLock<VerificationEngine>>,
}

/// Configuration for Chain-of-Thought reasoning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoTConfig {
    /// Maximum number of reasoning steps in the chain
    pub max_chain_length: u32,
    /// Enable verification of each reasoning step
    pub enable_verification: bool,
    /// Confidence threshold for accepting a reasoning step
    pub acceptance_threshold: f64,
    /// Number of verification attempts per step
    pub verification_attempts: u32,
    /// Enable backtracking when verification fails
    pub enable_backtracking: bool,
    /// Maximum number of backtrack attempts
    pub max_backtrack_attempts: u32,
    /// Enable alternative generation when backtracking
    pub enable_alternatives: bool,
    /// Timeout for the entire reasoning process
    pub reasoning_timeout: Duration,
    /// Enable step-by-step explanation
    pub enable_explanations: bool,
}

impl Default for CoTConfig {
    fn default() -> Self {
        Self {
            max_chain_length: 15,
            enable_verification: true,
            acceptance_threshold: 0.6,
            verification_attempts: 2,
            enable_backtracking: true,
            max_backtrack_attempts: 3,
            enable_alternatives: true,
            reasoning_timeout: Duration::from_secs(600), // 10 minutes
            enable_explanations: true,
        }
    }
}

/// A single step in the reasoning chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningStep {
    pub step_id: String,
    pub step_number: u32,
    pub premise: String,
    pub reasoning: String,
    pub conclusion: String,
    pub confidence: f64,
    pub verification_result: Option<VerificationResult>,
    pub alternatives: Vec<AlternativeStep>,
    pub created_at: SystemTime,
    pub backtrack_source: Option<String>, // ID of step we backtracked from
}

/// Alternative reasoning step when main reasoning fails
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlternativeStep {
    pub alt_id: String,
    pub alternative_reasoning: String,
    pub alternative_conclusion: String,
    pub confidence: f64,
    pub rationale: String, // Why this alternative was generated
}

/// Result of step verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub is_valid: bool,
    pub confidence: f64,
    pub issues_identified: Vec<String>,
    pub suggestions: Vec<String>,
    pub verification_reasoning: String,
}

/// The complete reasoning chain
#[derive(Debug)]
pub struct ReasoningChain {
    steps: Vec<ReasoningStep>,
    current_step: u32,
    backtrack_history: Vec<BacktrackEvent>,
    chain_confidence: f64,
    chain_coherence: f64,
    start_time: SystemTime,
}

impl Default for ReasoningChain {
    fn default() -> Self {
        Self {
            steps: Vec::new(),
            current_step: 0,
            backtrack_history: Vec::new(),
            chain_confidence: 0.0,
            chain_coherence: 0.0,
            start_time: SystemTime::now(),
        }
    }
}

/// Record of backtracking events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktrackEvent {
    pub event_id: String,
    pub from_step: u32,
    pub to_step: u32,
    pub reason: String,
    pub timestamp: SystemTime,
}

/// Engine for verifying reasoning steps
pub struct VerificationEngine {
    base_engine: Arc<dyn Engine>,
    verification_prompts: VerificationPrompts,
}

/// Prompts for different types of verification
#[derive(Debug, Clone)]
pub struct VerificationPrompts {
    pub logical_consistency: String,
    pub factual_accuracy: String,
    pub reasoning_validity: String,
    pub conclusion_support: String,
}

impl Default for VerificationPrompts {
    fn default() -> Self {
        Self {
            logical_consistency: "Evaluate the logical consistency of this reasoning step.".to_string(),
            factual_accuracy: "Check the factual accuracy of the claims made.".to_string(),
            reasoning_validity: "Assess whether the reasoning process is valid.".to_string(),
            conclusion_support: "Determine if the conclusion is well-supported by the premise.".to_string(),
        }
    }
}

/// Result of Chain-of-Thought reasoning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoTReasoningResult {
    pub reasoning_chain: Vec<ReasoningStep>,
    pub final_conclusion: String,
    pub chain_confidence: f64,
    pub verification_summary: String,
    pub backtrack_events: Vec<BacktrackEvent>,
    pub alternatives_explored: u32,
    pub reasoning_time: Duration,
    pub chain_quality_metrics: ChainQualityMetrics,
}

/// Metrics for evaluating chain quality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainQualityMetrics {
    pub coherence_score: f64,
    pub logical_consistency: f64,
    pub step_confidence_variance: f64,
    pub verification_pass_rate: f64,
    pub backtrack_frequency: f64,
}

impl ChainOfThoughtEngine {
    /// Create a new Chain-of-Thought reasoning engine
    pub fn new(base_engine: Arc<dyn Engine>, config: CoTConfig) -> Self {
        let verification_engine = Arc::new(RwLock::new(VerificationEngine::new(
            base_engine.clone(),
            VerificationPrompts::default(),
        )));

        Self {
            base_engine,
            config,
            reasoning_chain: Arc::new(RwLock::new(ReasoningChain::default())),
            verification_engine,
        }
    }

    /// Perform Chain-of-Thought reasoning on a problem
    pub async fn reason_with_chain(&self, problem: &str, context: &ExecutionContext) -> Result<CoTReasoningResult> {
        let start_time = SystemTime::now();
        
        // Initialize the reasoning chain
        self.initialize_chain(problem, context, start_time).await?;
        
        // Execute the reasoning process
        self.execute_reasoning_chain(problem, context, start_time).await?;
        
        // Generate final result
        let result = self.generate_chain_result(start_time).await?;
        
        Ok(result)
    }

    /// Initialize the reasoning chain with the initial problem
    async fn initialize_chain(&self, problem: &str, context: &ExecutionContext, start_time: SystemTime) -> Result<()> {
        let mut chain = self.reasoning_chain.write().await;
        
        chain.start_time = start_time;
        chain.current_step = 0;
        chain.steps.clear();
        chain.backtrack_history.clear();
        
        Ok(())
    }

    /// Execute the main reasoning chain process
    async fn execute_reasoning_chain(&self, problem: &str, context: &ExecutionContext, start_time: SystemTime) -> Result<()> {
        let mut current_premise = problem.to_string();
        
        for step_num in 1..=self.config.max_chain_length {
            // Check timeout
            if SystemTime::now().duration_since(start_time).unwrap_or_default() > self.config.reasoning_timeout {
                break;
            }

            // Generate reasoning step
            let step_result = self.generate_reasoning_step(step_num, &current_premise, context).await?;
            
            match step_result {
                StepResult::Success(step) => {
                    // Verify the step if verification is enabled
                    if self.config.enable_verification {
                        let verification = self.verify_step(&step).await?;
                        
                        if verification.is_valid && verification.confidence >= self.config.acceptance_threshold {
                            // Accept the step
                            current_premise = step.conclusion.clone();
                            self.add_step_to_chain(step, Some(verification)).await?;
                        } else {
                            // Step failed verification - try alternatives or backtrack
                            if self.config.enable_alternatives {
                                if let Some(alternative) = self.generate_alternative_step(&step, &verification).await? {
                                    current_premise = alternative.conclusion.clone();
                                    self.add_step_to_chain(alternative, Some(verification)).await?;
                                    continue;
                                }
                            }
                            
                            // Try backtracking
                            if self.config.enable_backtracking {
                                if let Some(backtrack_premise) = self.attempt_backtrack(step_num).await? {
                                    current_premise = backtrack_premise;
                                    continue;
                                }
                            }
                            
                            // If we can't recover, accept the step with low confidence
                            self.add_step_to_chain(step, Some(verification)).await?;
                        }
                    } else {
                        // No verification - accept the step
                        current_premise = step.conclusion.clone();
                        self.add_step_to_chain(step, None).await?;
                    }
                }
                
                StepResult::Failure(error) => {
                    // Failed to generate step - try backtracking
                    if self.config.enable_backtracking && step_num > 1 {
                        if let Some(backtrack_premise) = self.attempt_backtrack(step_num).await? {
                            current_premise = backtrack_premise;
                            continue;
                        }
                    }
                    
                    // Can't recover - break the chain
                    break;
                }
                
                StepResult::Complete => {
                    // Chain is complete
                    break;
                }
            }

            // Update chain metrics
            self.update_chain_metrics().await?;
        }

        Ok(())
    }

    /// Generate a single reasoning step
    async fn generate_reasoning_step(&self, step_num: u32, premise: &str, context: &ExecutionContext) -> Result<StepResult> {
        let prompt = format!(
            r#"Chain-of-Thought Reasoning - Step {}

Current premise: "{}"

Context: {}

Generate the next logical reasoning step. Provide:
1. Your reasoning process (how you think through this step)
2. The logical conclusion from this reasoning
3. Your confidence in this step (0.0-1.0)

If you believe the reasoning chain is complete and you have reached a final answer, indicate "COMPLETE" in your response.

Format your response as:
REASONING: [Your step-by-step reasoning]
CONCLUSION: [The logical conclusion]
CONFIDENCE: [0.0-1.0]
STATUS: [CONTINUE or COMPLETE]"#,
            step_num,
            premise,
            self.format_context_summary(context)
        );

        let request = fluent_core::types::Request {
            flowname: "chain_of_thought_step".to_string(),
            payload: prompt,
        };

        let response = std::pin::Pin::from(self.base_engine.execute(&request)).await?;
        self.parse_step_response(&response.content, step_num, premise)
    }

    /// Parse the LLM response into a reasoning step
    fn parse_step_response(&self, response: &str, step_num: u32, premise: &str) -> Result<StepResult> {
        let mut reasoning = String::new();
        let mut conclusion = String::new();
        let mut confidence = 0.5;
        let mut is_complete = false;

        for line in response.lines() {
            let line = line.trim();
            if line.starts_with("REASONING:") {
                reasoning = line.strip_prefix("REASONING:").unwrap_or("").trim().to_string();
            } else if line.starts_with("CONCLUSION:") {
                conclusion = line.strip_prefix("CONCLUSION:").unwrap_or("").trim().to_string();
            } else if line.starts_with("CONFIDENCE:") {
                if let Some(conf_str) = line.strip_prefix("CONFIDENCE:") {
                    confidence = (conf_str.trim().parse::<f64>().unwrap_or(0.5)).clamp(0.0, 1.0);
                }
            } else if line.starts_with("STATUS:") && line.contains("COMPLETE") {
                is_complete = true;
            }
        }

        if reasoning.is_empty() || conclusion.is_empty() {
            return Ok(StepResult::Failure("Failed to parse reasoning or conclusion".to_string()));
        }

        if is_complete {
            return Ok(StepResult::Complete);
        }

        let step = ReasoningStep {
            step_id: Uuid::new_v4().to_string(),
            step_number: step_num,
            premise: premise.to_string(),
            reasoning,
            conclusion,
            confidence,
            verification_result: None,
            alternatives: Vec::new(),
            created_at: SystemTime::now(),
            backtrack_source: None,
        };

        Ok(StepResult::Success(step))
    }

    /// Verify a reasoning step for correctness
    async fn verify_step(&self, step: &ReasoningStep) -> Result<VerificationResult> {
        let verification_engine = self.verification_engine.read().await;
        verification_engine.verify_reasoning_step(step).await
    }

    /// Generate an alternative step when verification fails
    async fn generate_alternative_step(&self, failed_step: &ReasoningStep, verification: &VerificationResult) -> Result<Option<ReasoningStep>> {
        if verification.suggestions.is_empty() {
            return Ok(None);
        }

        let prompt = format!(
            r#"The following reasoning step failed verification:

Premise: {}
Reasoning: {}
Conclusion: {}
Issues: {}
Suggestions: {}

Generate an alternative reasoning approach that addresses these issues:

Format your response as:
ALTERNATIVE_REASONING: [Your alternative reasoning]
ALTERNATIVE_CONCLUSION: [The alternative conclusion]  
CONFIDENCE: [0.0-1.0]
RATIONALE: [Why this alternative is better]"#,
            failed_step.premise,
            failed_step.reasoning,
            failed_step.conclusion,
            verification.issues_identified.join("; "),
            verification.suggestions.join("; ")
        );

        let request = fluent_core::types::Request {
            flowname: "chain_of_thought_alternative".to_string(),
            payload: prompt,
        };

        let response = std::pin::Pin::from(self.base_engine.execute(&request)).await?;
        
        // Parse alternative response
        let mut alt_reasoning = String::new();
        let mut alt_conclusion = String::new();
        let mut confidence = 0.5;
        let mut rationale = String::new();

        for line in response.content.lines() {
            let line = line.trim();
            if line.starts_with("ALTERNATIVE_REASONING:") {
                alt_reasoning = line.strip_prefix("ALTERNATIVE_REASONING:").unwrap_or("").trim().to_string();
            } else if line.starts_with("ALTERNATIVE_CONCLUSION:") {
                alt_conclusion = line.strip_prefix("ALTERNATIVE_CONCLUSION:").unwrap_or("").trim().to_string();
            } else if line.starts_with("CONFIDENCE:") {
                if let Some(conf_str) = line.strip_prefix("CONFIDENCE:") {
                    confidence = (conf_str.trim().parse::<f64>().unwrap_or(0.5)).clamp(0.0, 1.0);
                }
            } else if line.starts_with("RATIONALE:") {
                rationale = line.strip_prefix("RATIONALE:").unwrap_or("").trim().to_string();
            }
        }

        if alt_reasoning.is_empty() || alt_conclusion.is_empty() {
            return Ok(None);
        }

        let mut alternative_step = failed_step.clone();
        alternative_step.step_id = Uuid::new_v4().to_string();
        alternative_step.reasoning = alt_reasoning;
        alternative_step.conclusion = alt_conclusion;
        alternative_step.confidence = confidence;
        alternative_step.alternatives = vec![AlternativeStep {
            alt_id: Uuid::new_v4().to_string(),
            alternative_reasoning: failed_step.reasoning.clone(),
            alternative_conclusion: failed_step.conclusion.clone(),
            confidence: failed_step.confidence,
            rationale: "Original reasoning that was rejected".to_string(),
        }];

        Ok(Some(alternative_step))
    }

    /// Attempt to backtrack to a previous step
    async fn attempt_backtrack(&self, current_step: u32) -> Result<Option<String>> {
        if current_step <= 1 {
            return Ok(None);
        }

        // First, check backtrack history and find target
        let backtrack_target_data = {
            let chain = self.reasoning_chain.read().await;
            
            // Check backtrack history to avoid cycles
            let recent_backtracks = chain.backtrack_history.iter()
                .filter(|bt| SystemTime::now().duration_since(bt.timestamp).unwrap_or_default() < Duration::from_secs(300))
                .count();

            if recent_backtracks >= self.config.max_backtrack_attempts as usize {
                return Ok(None);
            }

            // Find a good backtrack point (step with high confidence)
            chain.steps.iter()
                .filter(|step| step.step_number < current_step && step.confidence > 0.7)
                .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap_or(std::cmp::Ordering::Equal))
                .map(|step| (step.step_number, step.conclusion.clone()))
        };

        if let Some((target_step_number, target_conclusion)) = backtrack_target_data {
            // Record backtrack event and update chain
            let mut chain = self.reasoning_chain.write().await;
            
            let backtrack_event = BacktrackEvent {
                event_id: Uuid::new_v4().to_string(),
                from_step: current_step,
                to_step: target_step_number,
                reason: "Verification failure - seeking higher confidence path".to_string(),
                timestamp: SystemTime::now(),
            };
            
            chain.backtrack_history.push(backtrack_event);
            
            // Truncate chain to backtrack point
            chain.steps.truncate(target_step_number as usize);
            chain.current_step = target_step_number;
            
            Ok(Some(target_conclusion))
        } else {
            Ok(None)
        }
    }

    /// Add a step to the reasoning chain
    async fn add_step_to_chain(&self, mut step: ReasoningStep, verification: Option<VerificationResult>) -> Result<()> {
        let mut chain = self.reasoning_chain.write().await;
        
        step.verification_result = verification;
        chain.steps.push(step);
        chain.current_step += 1;
        
        Ok(())
    }

    /// Update chain quality metrics
    async fn update_chain_metrics(&self) -> Result<()> {
        let mut chain = self.reasoning_chain.write().await;
        
        if chain.steps.is_empty() {
            return Ok(());
        }

        // Calculate average confidence
        let total_confidence: f64 = chain.steps.iter().map(|s| s.confidence).sum();
        chain.chain_confidence = total_confidence / chain.steps.len() as f64;
        
        // Calculate coherence (simplified metric)
        let coherence_scores: Vec<f64> = chain.steps.windows(2)
            .map(|pair| self.calculate_step_coherence(&pair[0], &pair[1]))
            .collect();
            
        if !coherence_scores.is_empty() {
            chain.chain_coherence = coherence_scores.iter().sum::<f64>() / coherence_scores.len() as f64;
        }
        
        Ok(())
    }

    /// Calculate coherence between two consecutive steps
    fn calculate_step_coherence(&self, step1: &ReasoningStep, step2: &ReasoningStep) -> f64 {
        // Simplified coherence: check if step2's premise matches step1's conclusion
        if step2.premise.contains(&step1.conclusion) || step1.conclusion.contains(&step2.premise) {
            0.9
        } else {
            0.5 // Neutral coherence
        }
    }

    /// Generate final reasoning result
    async fn generate_chain_result(&self, start_time: SystemTime) -> Result<CoTReasoningResult> {
        let chain = self.reasoning_chain.read().await;
        
        let final_conclusion = chain.steps.last()
            .map(|step| step.conclusion.clone())
            .unwrap_or_else(|| "No conclusion reached".to_string());

        let verification_summary = self.generate_verification_summary(&chain).await;
        
        let quality_metrics = ChainQualityMetrics {
            coherence_score: chain.chain_coherence,
            logical_consistency: self.calculate_logical_consistency(&chain).await,
            step_confidence_variance: self.calculate_confidence_variance(&chain),
            verification_pass_rate: self.calculate_verification_pass_rate(&chain),
            backtrack_frequency: chain.backtrack_history.len() as f64 / chain.steps.len().max(1) as f64,
        };

        Ok(CoTReasoningResult {
            reasoning_chain: chain.steps.clone(),
            final_conclusion,
            chain_confidence: chain.chain_confidence,
            verification_summary,
            backtrack_events: chain.backtrack_history.clone(),
            alternatives_explored: chain.steps.iter()
                .map(|s| s.alternatives.len() as u32)
                .sum(),
            reasoning_time: SystemTime::now().duration_since(start_time).unwrap_or_default(),
            chain_quality_metrics: quality_metrics,
        })
    }

    // Helper methods
    
    async fn generate_verification_summary(&self, chain: &ReasoningChain) -> String {
        let total_steps = chain.steps.len();
        let verified_steps = chain.steps.iter()
            .filter(|s| s.verification_result.is_some())
            .count();
        let passed_steps = chain.steps.iter()
            .filter(|s| s.verification_result.as_ref().map(|v| v.is_valid).unwrap_or(false))
            .count();

        format!(
            "Verification: {}/{} steps verified, {}/{} passed verification",
            verified_steps, total_steps, passed_steps, verified_steps
        )
    }

    async fn calculate_logical_consistency(&self, chain: &ReasoningChain) -> f64 {
        // Simplified consistency check
        let consistent_steps = chain.steps.windows(2)
            .filter(|pair| self.calculate_step_coherence(&pair[0], &pair[1]) > 0.7)
            .count();
        
        if chain.steps.len() <= 1 {
            1.0
        } else {
            consistent_steps as f64 / (chain.steps.len() - 1) as f64
        }
    }

    fn calculate_confidence_variance(&self, chain: &ReasoningChain) -> f64 {
        if chain.steps.is_empty() {
            return 0.0;
        }

        let mean = chain.chain_confidence;
        let variance: f64 = chain.steps.iter()
            .map(|step| (step.confidence - mean).powi(2))
            .sum::<f64>() / chain.steps.len() as f64;
        
        variance
    }

    fn calculate_verification_pass_rate(&self, chain: &ReasoningChain) -> f64 {
        let verified_steps: Vec<_> = chain.steps.iter()
            .filter_map(|s| s.verification_result.as_ref())
            .collect();

        if verified_steps.is_empty() {
            return 0.0;
        }

        let passed_steps = verified_steps.iter()
            .filter(|v| v.is_valid)
            .count();

        passed_steps as f64 / verified_steps.len() as f64
    }

    fn format_context_summary(&self, context: &ExecutionContext) -> String {
        format!(
            "Goal: {}, Context items: {}, Iteration: {}",
            context.current_goal.as_ref()
                .map(|g| g.description.clone())
                .unwrap_or_else(|| "No goal set".to_string()),
            context.context_data.len(),
            context.iteration_count
        )
    }
}

/// Result of attempting to generate a reasoning step
enum StepResult {
    Success(ReasoningStep),
    Failure(String),
    Complete,
}

impl VerificationEngine {
    fn new(base_engine: Arc<dyn Engine>, prompts: VerificationPrompts) -> Self {
        Self {
            base_engine,
            verification_prompts: prompts,
        }
    }

    async fn verify_reasoning_step(&self, step: &ReasoningStep) -> Result<VerificationResult> {
        let prompt = format!(
            r#"Verify this reasoning step:

Premise: {}
Reasoning: {}  
Conclusion: {}
Confidence: {}

Evaluate:
1. Is the reasoning logically valid?
2. Does the conclusion follow from the premise and reasoning?
3. Are there any logical fallacies or errors?
4. Is the confidence level appropriate?

Provide:
- VALID: true/false
- CONFIDENCE: 0.0-1.0 (your confidence in the verification)
- ISSUES: list any problems found
- SUGGESTIONS: recommendations for improvement"#,
            step.premise,
            step.reasoning,
            step.conclusion,
            step.confidence
        );

        let request = fluent_core::types::Request {
            flowname: "chain_verification".to_string(),
            payload: prompt,
        };

        let response = std::pin::Pin::from(self.base_engine.execute(&request)).await?;
        self.parse_verification_response(&response.content)
    }

    fn parse_verification_response(&self, response: &str) -> Result<VerificationResult> {
        let mut is_valid = false;
        let mut confidence = 0.5;
        let mut issues = Vec::new();
        let mut suggestions = Vec::new();

        for line in response.lines() {
            let line = line.trim();
            if line.starts_with("VALID:") && line.contains("true") {
                is_valid = true;
            } else if line.starts_with("CONFIDENCE:") {
                if let Some(conf_str) = line.strip_prefix("CONFIDENCE:") {
                    confidence = (conf_str.trim().parse::<f64>().unwrap_or(0.5)).clamp(0.0, 1.0);
                }
            } else if line.starts_with("ISSUES:") {
                if let Some(issues_str) = line.strip_prefix("ISSUES:") {
                    issues.push(issues_str.trim().to_string());
                }
            } else if line.starts_with("SUGGESTIONS:") {
                if let Some(sugg_str) = line.strip_prefix("SUGGESTIONS:") {
                    suggestions.push(sugg_str.trim().to_string());
                }
            }
        }

        Ok(VerificationResult {
            is_valid,
            confidence,
            issues_identified: issues,
            suggestions,
            verification_reasoning: response.to_string(),
        })
    }
}

#[async_trait::async_trait]
impl ReasoningEngine for ChainOfThoughtEngine {
    async fn reason(&self, prompt: &str, context: &ExecutionContext) -> Result<String> {
        let result = self.reason_with_chain(prompt, context).await?;
        
        let summary = format!(
            "Chain-of-Thought Reasoning Result:\n\nReasoning Chain ({} steps):\n{}\n\nFinal Conclusion: {}\n\nConfidence: {:.2}\nVerification: {}\nBacktracks: {}",
            result.reasoning_chain.len(),
            result.reasoning_chain.iter()
                .map(|step| format!("Step {}: {} -> {}", step.step_number, step.reasoning, step.conclusion))
                .collect::<Vec<_>>()
                .join("\n"),
            result.final_conclusion,
            result.chain_confidence,
            result.verification_summary,
            result.backtrack_events.len()
        );
        
        Ok(summary)
    }

    async fn get_capabilities(&self) -> Vec<ReasoningCapability> {
        vec![
            ReasoningCapability::ChainOfThought,
            ReasoningCapability::ConfidenceScoring,
            ReasoningCapability::ErrorAnalysis,
            ReasoningCapability::BacktrackingSearch,
            ReasoningCapability::ProblemSolving,
        ]
    }

    async fn get_confidence(&self) -> f64 {
        let chain = self.reasoning_chain.read().await;
        chain.chain_confidence
    }
}