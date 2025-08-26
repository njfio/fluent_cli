//! Advanced reasoning engines for sophisticated problem solving
//!
//! This module contains various reasoning engines that implement different
//! cognitive patterns for autonomous problem solving.

pub mod tree_of_thought;
pub mod chain_of_thought;
pub mod meta_reasoning;
pub mod enhanced_multi_modal;

pub use tree_of_thought::{TreeOfThoughtEngine, ToTConfig, ToTReasoningResult};
pub use chain_of_thought::{ChainOfThoughtEngine, CoTConfig, CoTReasoningResult};
pub use meta_reasoning::{MetaReasoningEngine, MetaConfig, MetaReasoningResult};
pub use enhanced_multi_modal::{EnhancedMultiModalEngine, EnhancedReasoningConfig, EnhancedReasoningResult};

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
    pub fn new(engines: Vec<Box<dyn ReasoningEngine>>, strategy: ReasoningSelectionStrategy) -> Self {
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
                    if let Some(tot_engine) = self.engines.iter().find(|e| {
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
                        results.into_iter()
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