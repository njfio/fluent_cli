//! Meta-reasoning Engine
//!
//! This module implements meta-reasoning capabilities that enable the agent
//! to reason about its own reasoning processes, evaluate strategies, and
//! adapt its approach based on performance feedback.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::reasoning::{ReasoningEngine, ReasoningCapability};
use crate::context::ExecutionContext;
use fluent_core::traits::Engine;

/// Meta-reasoning engine for strategy evaluation and adaptation
pub struct MetaReasoningEngine {
    base_engine: Arc<dyn Engine>,
    config: MetaConfig,
    strategy_history: Arc<RwLock<StrategyHistory>>,
    performance_tracker: Arc<RwLock<PerformanceTracker>>,
}

/// Configuration for meta-reasoning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaConfig {
    pub enable_strategy_evaluation: bool,
    pub enable_performance_tracking: bool,
    pub adaptation_threshold: f64,
    pub strategy_window_size: u32,
    pub min_samples_for_adaptation: u32,
}

impl Default for MetaConfig {
    fn default() -> Self {
        Self {
            enable_strategy_evaluation: true,
            enable_performance_tracking: true,
            adaptation_threshold: 0.3,
            strategy_window_size: 10,
            min_samples_for_adaptation: 3,
        }
    }
}

/// Strategy performance tracking
#[derive(Debug, Default)]
pub struct StrategyHistory {
    strategies: HashMap<String, StrategyRecord>,
    current_strategy: Option<String>,
    adaptation_events: Vec<AdaptationEvent>,
}

/// Record of a reasoning strategy's performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyRecord {
    pub strategy_id: String,
    pub strategy_type: StrategyType,
    pub usage_count: u32,
    pub success_rate: f64,
    pub average_confidence: f64,
    pub average_time: Duration,
    pub performance_trend: Vec<f64>,
    pub last_used: SystemTime,
}

/// Types of reasoning strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StrategyType {
    TreeOfThought,
    ChainOfThought,
    DirectReasoning,
    Composite,
    Adaptive,
}

/// Performance tracking system
#[derive(Debug, Default)]
pub struct PerformanceTracker {
    recent_performances: Vec<PerformanceMetric>,
    current_baseline: f64,
    adaptation_suggestions: Vec<AdaptationSuggestion>,
}

/// Individual performance measurement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetric {
    pub timestamp: SystemTime,
    pub strategy_used: String,
    pub confidence: f64,
    pub success: bool,
    pub execution_time: Duration,
    pub complexity_score: f64,
}

/// Strategy adaptation event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptationEvent {
    pub event_id: String,
    pub timestamp: SystemTime,
    pub from_strategy: String,
    pub to_strategy: String,
    pub reason: String,
    pub performance_improvement: f64,
}

/// Suggestion for strategy adaptation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptationSuggestion {
    pub suggestion_id: String,
    pub suggested_strategy: StrategyType,
    pub confidence: f64,
    pub rationale: String,
    pub expected_improvement: f64,
}

/// Meta-reasoning result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaReasoningResult {
    pub strategy_evaluation: StrategyEvaluation,
    pub performance_analysis: PerformanceAnalysis,
    pub adaptation_recommendations: Vec<AdaptationSuggestion>,
    pub meta_confidence: f64,
}

/// Evaluation of current reasoning strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyEvaluation {
    pub current_strategy_effectiveness: f64,
    pub strategy_appropriateness: f64,
    pub improvement_potential: f64,
    pub alternative_strategies: Vec<String>,
}

/// Analysis of reasoning performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceAnalysis {
    pub current_performance_level: f64,
    pub performance_trend: PerformanceTrend,
    pub bottlenecks_identified: Vec<String>,
    pub strengths_identified: Vec<String>,
}

/// Performance trend analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PerformanceTrend {
    Improving,
    Declining,
    Stable,
    Volatile,
}

impl MetaReasoningEngine {
    /// Create a new meta-reasoning engine
    pub fn new(base_engine: Arc<dyn Engine>, config: MetaConfig) -> Self {
        Self {
            base_engine,
            config,
            strategy_history: Arc::new(RwLock::new(StrategyHistory::default())),
            performance_tracker: Arc::new(RwLock::new(PerformanceTracker::default())),
        }
    }

    /// Perform meta-reasoning analysis
    pub async fn meta_reason(&self, context: &ExecutionContext, recent_reasoning: &str) -> Result<MetaReasoningResult> {
        // Evaluate current strategy
        let strategy_eval = self.evaluate_current_strategy(context, recent_reasoning).await?;
        
        // Analyze performance trends
        let performance_analysis = self.analyze_performance_trends().await?;
        
        // Generate adaptation recommendations
        let recommendations = self.generate_adaptation_recommendations(&strategy_eval, &performance_analysis).await?;
        
        // Calculate meta-confidence
        let meta_confidence = self.calculate_meta_confidence(&strategy_eval, &performance_analysis).await;

        Ok(MetaReasoningResult {
            strategy_evaluation: strategy_eval,
            performance_analysis,
            adaptation_recommendations: recommendations,
            meta_confidence,
        })
    }

    /// Evaluate the effectiveness of the current reasoning strategy
    async fn evaluate_current_strategy(&self, context: &ExecutionContext, recent_reasoning: &str) -> Result<StrategyEvaluation> {
        let prompt = format!(
            r#"Evaluate this reasoning approach:

Recent reasoning: {}
Context: {}

Assess:
1. How effective is this reasoning approach? (0.0-1.0)
2. Is this approach appropriate for the problem type? (0.0-1.0)  
3. What improvement potential exists? (0.0-1.0)
4. What alternative approaches could work better?

Format:
EFFECTIVENESS: [0.0-1.0]
APPROPRIATENESS: [0.0-1.0]
IMPROVEMENT_POTENTIAL: [0.0-1.0]
ALTERNATIVES: [list alternative approaches]"#,
            recent_reasoning,
            self.format_context_summary(context)
        );

        let request = fluent_core::types::Request {
            flowname: "meta_strategy_evaluation".to_string(),
            payload: prompt,
        };

        let response = std::pin::Pin::from(self.base_engine.execute(&request)).await?;
        self.parse_strategy_evaluation(&response.content)
    }

    /// Analyze performance trends from recent executions
    async fn analyze_performance_trends(&self) -> Result<PerformanceAnalysis> {
        let tracker = self.performance_tracker.read().await;
        
        if tracker.recent_performances.len() < 3 {
            return Ok(PerformanceAnalysis {
                current_performance_level: 0.5,
                performance_trend: PerformanceTrend::Stable,
                bottlenecks_identified: Vec::new(),
                strengths_identified: Vec::new(),
            });
        }

        // Calculate performance metrics
        let recent_scores: Vec<f64> = tracker.recent_performances.iter()
            .map(|p| if p.success { p.confidence } else { 0.0 })
            .collect();

        let current_level = recent_scores.iter().sum::<f64>() / recent_scores.len() as f64;
        
        // Determine trend
        let trend = if recent_scores.len() >= 5 {
            let first_half: f64 = recent_scores[..recent_scores.len()/2].iter().sum::<f64>() / (recent_scores.len()/2) as f64;
            let second_half: f64 = recent_scores[recent_scores.len()/2..].iter().sum::<f64>() / (recent_scores.len() - recent_scores.len()/2) as f64;
            
            if second_half - first_half > 0.1 {
                PerformanceTrend::Improving
            } else if first_half - second_half > 0.1 {
                PerformanceTrend::Declining
            } else {
                PerformanceTrend::Stable
            }
        } else {
            PerformanceTrend::Stable
        };

        Ok(PerformanceAnalysis {
            current_performance_level: current_level,
            performance_trend: trend,
            bottlenecks_identified: self.identify_bottlenecks(&tracker.recent_performances).await,
            strengths_identified: self.identify_strengths(&tracker.recent_performances).await,
        })
    }

    /// Generate recommendations for strategy adaptation
    async fn generate_adaptation_recommendations(&self, strategy_eval: &StrategyEvaluation, performance_analysis: &PerformanceAnalysis) -> Result<Vec<AdaptationSuggestion>> {
        let mut recommendations = Vec::new();

        // If current strategy is underperforming, suggest alternatives
        if strategy_eval.current_strategy_effectiveness < self.config.adaptation_threshold {
            recommendations.push(AdaptationSuggestion {
                suggestion_id: Uuid::new_v4().to_string(),
                suggested_strategy: StrategyType::TreeOfThought,
                confidence: 0.8,
                rationale: "Current strategy shows low effectiveness - Tree-of-Thought may provide better exploration".to_string(),
                expected_improvement: 0.3,
            });
        }

        // If performance is declining, suggest more robust approach
        if matches!(performance_analysis.performance_trend, PerformanceTrend::Declining) {
            recommendations.push(AdaptationSuggestion {
                suggestion_id: Uuid::new_v4().to_string(),
                suggested_strategy: StrategyType::Composite,
                confidence: 0.7,
                rationale: "Declining performance trend - composite approach may provide better resilience".to_string(),
                expected_improvement: 0.25,
            });
        }

        Ok(recommendations)
    }

    /// Calculate confidence in meta-reasoning analysis
    async fn calculate_meta_confidence(&self, strategy_eval: &StrategyEvaluation, performance_analysis: &PerformanceAnalysis) -> f64 {
        let tracker = self.performance_tracker.read().await;
        let sample_size_factor = (tracker.recent_performances.len() as f64 / 10.0).min(1.0);
        let evaluation_consistency = (strategy_eval.current_strategy_effectiveness + strategy_eval.strategy_appropriateness) / 2.0;
        
        (sample_size_factor * 0.4 + evaluation_consistency * 0.6).clamp(0.0, 1.0)
    }

    // Helper methods
    
    fn parse_strategy_evaluation(&self, response: &str) -> Result<StrategyEvaluation> {
        let mut effectiveness = 0.5;
        let mut appropriateness = 0.5;
        let mut improvement_potential = 0.5;
        let mut alternatives = Vec::new();

        for line in response.lines() {
            let line = line.trim();
            if line.starts_with("EFFECTIVENESS:") {
                if let Some(val_str) = line.strip_prefix("EFFECTIVENESS:") {
                    effectiveness = (val_str.trim().parse::<f64>().unwrap_or(0.5)).clamp(0.0, 1.0);
                }
            } else if line.starts_with("APPROPRIATENESS:") {
                if let Some(val_str) = line.strip_prefix("APPROPRIATENESS:") {
                    appropriateness = (val_str.trim().parse::<f64>().unwrap_or(0.5)).clamp(0.0, 1.0);
                }
            } else if line.starts_with("IMPROVEMENT_POTENTIAL:") {
                if let Some(val_str) = line.strip_prefix("IMPROVEMENT_POTENTIAL:") {
                    improvement_potential = (val_str.trim().parse::<f64>().unwrap_or(0.5)).clamp(0.0, 1.0);
                }
            } else if line.starts_with("ALTERNATIVES:") {
                if let Some(alts_str) = line.strip_prefix("ALTERNATIVES:") {
                    alternatives = alts_str.split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
            }
        }

        Ok(StrategyEvaluation {
            current_strategy_effectiveness: effectiveness,
            strategy_appropriateness: appropriateness,
            improvement_potential,
            alternative_strategies: alternatives,
        })
    }

    async fn identify_bottlenecks(&self, performances: &[PerformanceMetric]) -> Vec<String> {
        let mut bottlenecks = Vec::new();
        
        // Check for time bottlenecks
        let avg_time: Duration = performances.iter()
            .map(|p| p.execution_time)
            .sum::<Duration>() / performances.len() as u32;
            
        if avg_time > Duration::from_secs(60) {
            bottlenecks.push("Long execution times detected".to_string());
        }

        // Check for confidence issues
        let avg_confidence: f64 = performances.iter()
            .map(|p| p.confidence)
            .sum::<f64>() / performances.len() as f64;
            
        if avg_confidence < 0.6 {
            bottlenecks.push("Low confidence in reasoning outputs".to_string());
        }

        bottlenecks
    }

    async fn identify_strengths(&self, performances: &[PerformanceMetric]) -> Vec<String> {
        let mut strengths = Vec::new();
        
        let success_rate = performances.iter()
            .filter(|p| p.success)
            .count() as f64 / performances.len() as f64;

        if success_rate > 0.8 {
            strengths.push("High success rate maintained".to_string());
        }

        let avg_confidence: f64 = performances.iter()
            .map(|p| p.confidence)
            .sum::<f64>() / performances.len() as f64;

        if avg_confidence > 0.8 {
            strengths.push("Consistently high confidence levels".to_string());
        }

        strengths
    }

    fn format_context_summary(&self, context: &ExecutionContext) -> String {
        format!(
            "Goal: {}, Iteration: {}, Context items: {}",
            context.current_goal.as_ref()
                .map(|g| g.description.clone())
                .unwrap_or_else(|| "No goal set".to_string()),
            context.iteration_count,
            context.context_data.len()
        )
    }

    /// Record performance for meta-analysis
    pub async fn record_performance(&self, metric: PerformanceMetric) -> Result<()> {
        let mut tracker = self.performance_tracker.write().await;
        tracker.recent_performances.push(metric);
        
        // Keep only recent performances
        let window_size = self.config.strategy_window_size as usize;
        if tracker.recent_performances.len() > window_size {
            let excess = tracker.recent_performances.len() - window_size;
            tracker.recent_performances.drain(0..excess);
        }
        
        Ok(())
    }
}

#[async_trait::async_trait]
impl ReasoningEngine for MetaReasoningEngine {
    async fn reason(&self, prompt: &str, context: &ExecutionContext) -> Result<String> {
        let result = self.meta_reason(context, prompt).await?;
        
        let summary = format!(
            "Meta-reasoning Analysis:\n\nStrategy Effectiveness: {:.2}\nPerformance Level: {:.2}\nTrend: {:?}\nRecommendations: {}\nMeta-confidence: {:.2}",
            result.strategy_evaluation.current_strategy_effectiveness,
            result.performance_analysis.current_performance_level,
            result.performance_analysis.performance_trend,
            result.adaptation_recommendations.len(),
            result.meta_confidence
        );
        
        Ok(summary)
    }

    async fn get_capabilities(&self) -> Vec<ReasoningCapability> {
        vec![
            ReasoningCapability::MetaCognition,
            ReasoningCapability::StrategyFormulation,
            ReasoningCapability::PerformanceEvaluation,
            ReasoningCapability::SelfReflection,
        ]
    }

    async fn get_confidence(&self) -> f64 {
        let tracker = self.performance_tracker.read().await;
        if tracker.recent_performances.is_empty() {
            0.5
        } else {
            tracker.current_baseline
        }
    }
}