//! Type definitions for the reflection system
//! 
//! This module contains all the data structures and enums used throughout
//! the reflection system for analysis, strategy adjustments, and learning.

use serde::{Deserialize, Serialize};
use std::time::Duration;

// Re-export types from the main reflection engine module
pub use crate::reflection_engine::{
    ReflectionConfig, ReflectionResult, ReflectionType, ReflectionTrigger,
    ReflectionAnalysis, ProgressAssessment, StrategyEffectiveness,
    LearningOpportunity, Bottleneck, SuccessPattern, FailurePattern,
    ResourceUtilization, VelocityTrend, MilestoneAchievement, QualityMetrics,
    StrategyAdjustment, AdjustmentType, LearningInsight, InsightType,
    Applicability, Recommendation, RecommendationType, Priority, Urgency,
    ImpactLevel, DifficultyLevel, LearningType, LearningExperience,
    StrategyPattern, PerformanceMetrics,
};

/// Optimization plan for strategy improvements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationPlan {
    pub plan_id: String,
    pub prioritized_adjustments: Vec<PrioritizedAdjustment>,
    pub implementation_timeline: Vec<TimelinePhase>,
    pub resource_requirements: ResourceRequirements,
    pub risk_assessment: Vec<ImplementationRisk>,
    pub success_criteria: Vec<SuccessCriterion>,
    pub monitoring_plan: MonitoringPlan,
}

/// Prioritized strategy adjustment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrioritizedAdjustment {
    pub adjustment: StrategyAdjustment,
    pub priority_score: f64,
    pub execution_order: usize,
}

/// Timeline phase for implementation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelinePhase {
    pub phase_id: String,
    pub description: String,
    pub duration: Duration,
    pub dependencies: Vec<String>,
    pub deliverables: Vec<String>,
}

/// Resource requirements for implementation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub computational_resources: f64,
    pub time_investment: Duration,
    pub human_oversight_required: bool,
    pub external_dependencies: Vec<String>,
}

/// Implementation risk assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplementationRisk {
    pub risk_id: String,
    pub description: String,
    pub probability: f64,
    pub impact: ImpactLevel,
    pub mitigation_strategy: String,
}

/// Success criterion for evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessCriterion {
    pub criterion_id: String,
    pub description: String,
    pub measurement_method: String,
    pub target_value: f64,
    pub evaluation_timeline: Duration,
}

/// Monitoring plan for tracking progress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringPlan {
    pub monitoring_frequency: Duration,
    pub key_metrics: Vec<String>,
    pub alert_thresholds: Vec<AlertThreshold>,
    pub review_schedule: Duration,
}

/// Alert threshold for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThreshold {
    pub metric: String,
    pub threshold: f64,
    pub action: String,
}

/// Retention priority for learning insights
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPriority {
    pub insight_id: String,
    pub priority_score: f64,
    pub retention_strategy: RetentionStrategy,
    pub review_frequency: Duration,
}

/// Retention strategy for knowledge management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RetentionStrategy {
    ReinforcementLearning,
    AvoidancePattern,
    BestPractice,
    StrategicKnowledge,
    OperationalKnowledge,
    ContextualMemory,
    ProcessImprovement,
}

/// Prioritized recommendation with additional metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrioritizedRecommendation {
    pub recommendation: Recommendation,
    pub adjusted_priority: Priority,
    pub feasibility_score: f64,
    pub impact_score: f64,
    pub execution_order: usize,
    pub dependencies: Vec<String>,
}

/// Context constraints for recommendation prioritization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConstraints {
    pub time_pressure: bool,
    pub resource_limitations: bool,
    pub complexity_limitations: bool,
    pub risk_tolerance: f64,
}

/// Performance assessment result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceAssessment {
    pub overall_score: f64,
    pub efficiency_score: f64,
    pub quality_score: f64,
    pub consistency_score: f64,
    pub improvement_trend: TrendDirection,
}

/// Confidence assessment result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceAssessment {
    pub overall_confidence: f64,
    pub strategy_confidence: f64,
    pub execution_confidence: f64,
    pub outcome_confidence: f64,
    pub confidence_trend: TrendDirection,
}

/// Trend direction indicator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrendDirection {
    Improving,
    Stable,
    Declining,
    Volatile,
}

/// Reflection coordinator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionCoordinatorConfig {
    pub analysis_depth: AnalysisDepth,
    pub strategy_sensitivity: f64,
    pub learning_aggressiveness: f64,
    pub recommendation_threshold: f64,
}

/// Analysis depth level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnalysisDepth {
    Surface,    // Basic metrics only
    Standard,   // Normal analysis
    Deep,       // Comprehensive analysis
    Exhaustive, // Maximum depth analysis
}

/// Reflection execution context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionExecutionContext {
    pub trigger_context: String,
    pub available_time: Duration,
    pub resource_constraints: Vec<String>,
    pub priority_focus: Vec<String>,
}

/// Learning insight categorization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsightCategory {
    pub category_name: String,
    pub insights: Vec<LearningInsight>,
    pub category_confidence: f64,
    pub applicability_scope: Vec<String>,
}

/// Strategy adjustment impact assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdjustmentImpactAssessment {
    pub adjustment_id: String,
    pub predicted_impact: f64,
    pub confidence_level: f64,
    pub risk_factors: Vec<String>,
    pub success_indicators: Vec<String>,
}

/// Reflection quality metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionQualityMetrics {
    pub analysis_completeness: f64,
    pub insight_relevance: f64,
    pub recommendation_actionability: f64,
    pub strategy_alignment: f64,
    pub learning_value: f64,
}

/// Meta-reflection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaReflectionResult {
    pub reflection_effectiveness: f64,
    pub process_improvements: Vec<String>,
    pub methodology_adjustments: Vec<String>,
    pub quality_assessment: ReflectionQualityMetrics,
}

/// Crisis reflection context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrisisReflectionContext {
    pub crisis_severity: ImpactLevel,
    pub immediate_actions_required: Vec<String>,
    pub recovery_timeline: Duration,
    pub stakeholder_impact: Vec<String>,
}

/// Reflection session summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionSessionSummary {
    pub session_id: String,
    pub duration: Duration,
    pub insights_generated: usize,
    pub adjustments_proposed: usize,
    pub recommendations_made: usize,
    pub confidence_change: f64,
    pub performance_change: f64,
}



impl Default for ContextConstraints {
    fn default() -> Self {
        Self {
            time_pressure: false,
            resource_limitations: false,
            complexity_limitations: false,
            risk_tolerance: 0.5,
        }
    }
}

impl Default for ReflectionCoordinatorConfig {
    fn default() -> Self {
        Self {
            analysis_depth: AnalysisDepth::Standard,
            strategy_sensitivity: 0.7,
            learning_aggressiveness: 0.6,
            recommendation_threshold: 0.5,
        }
    }
}
