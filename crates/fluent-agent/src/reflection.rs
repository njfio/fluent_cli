use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use crate::context::{ExecutionContext, ExecutionEventType};
use crate::reasoning::ReasoningEngine;

/// Advanced self-reflection system for agent learning and strategy adjustment
pub struct ReflectionEngine {
    reflection_config: ReflectionConfig,
    learning_history: Vec<LearningExperience>,
    strategy_patterns: Vec<StrategyPattern>,
    performance_metrics: PerformanceMetrics,
}

/// Configuration for reflection behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionConfig {
    pub reflection_frequency: u32, // Reflect every N iterations
    pub deep_reflection_frequency: u32, // Deep reflection every N reflections
    pub learning_retention_days: u32,
    pub confidence_threshold: f64, // Trigger reflection if confidence drops below this
    pub performance_threshold: f64, // Trigger strategy adjustment if performance drops below this
    pub enable_meta_reflection: bool, // Reflect on the reflection process itself
    pub strategy_adjustment_sensitivity: f64, // How readily to adjust strategy (0.0-1.0)
}

/// Result of a reflection process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionResult {
    pub reflection_id: String,
    pub timestamp: SystemTime,
    pub reflection_type: ReflectionType,
    pub trigger_reason: ReflectionTrigger,
    pub analysis: ReflectionAnalysis,
    pub strategy_adjustments: Vec<StrategyAdjustment>,
    pub learning_insights: Vec<LearningInsight>,
    pub confidence_assessment: f64,
    pub performance_assessment: f64,
    pub recommendations: Vec<Recommendation>,
}

/// Types of reflection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReflectionType {
    Routine,        // Regular scheduled reflection
    Triggered,      // Triggered by low confidence or performance
    Deep,           // Comprehensive analysis of patterns and strategies
    Meta,           // Reflection on the reflection process itself
    Crisis,         // Emergency reflection due to critical failures
}

/// What triggered the reflection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReflectionTrigger {
    ScheduledInterval,
    LowConfidence(f64),
    PoorPerformance(f64),
    RepeatedFailures(u32),
    GoalStagnation,
    UserRequest,
    CriticalError(String),
}

/// Comprehensive analysis from reflection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionAnalysis {
    pub progress_assessment: ProgressAssessment,
    pub strategy_effectiveness: StrategyEffectiveness,
    pub learning_opportunities: Vec<LearningOpportunity>,
    pub bottlenecks_identified: Vec<Bottleneck>,
    pub success_patterns: Vec<SuccessPattern>,
    pub failure_patterns: Vec<FailurePattern>,
    pub resource_utilization: ResourceUtilization,
}

/// Assessment of current progress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressAssessment {
    pub goal_completion_percentage: f64,
    pub velocity_trend: VelocityTrend,
    pub milestone_achievements: Vec<MilestoneAchievement>,
    pub time_efficiency: f64,
    pub quality_metrics: QualityMetrics,
}

/// Strategy effectiveness evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyEffectiveness {
    pub current_strategy_score: f64,
    pub strategy_consistency: f64,
    pub adaptation_frequency: f64,
    pub strategy_alignment: f64, // How well strategy aligns with goal
    pub execution_quality: f64,
}

/// Learning opportunity identified
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningOpportunity {
    pub opportunity_id: String,
    pub description: String,
    pub potential_impact: ImpactLevel,
    pub learning_type: LearningType,
    pub implementation_difficulty: DifficultyLevel,
    pub priority: Priority,
}

/// Types of learning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LearningType {
    SkillImprovement,
    StrategyOptimization,
    ToolUsageEnhancement,
    PatternRecognition,
    ErrorPrevention,
    EfficiencyGain,
}

/// Strategy adjustment recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyAdjustment {
    pub adjustment_id: String,
    pub adjustment_type: AdjustmentType,
    pub description: String,
    pub rationale: String,
    pub expected_impact: ImpactLevel,
    pub implementation_steps: Vec<String>,
    pub success_metrics: Vec<String>,
    pub rollback_plan: Option<String>,
}

/// Types of strategy adjustments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AdjustmentType {
    GoalRefinement,
    TaskPrioritization,
    ToolSelection,
    ApproachModification,
    ResourceReallocation,
    TimelineAdjustment,
    QualityStandards,
    RiskManagement,
    StrategyOptimization,
}

impl std::fmt::Display for AdjustmentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdjustmentType::GoalRefinement => write!(f, "Goal Refinement"),
            AdjustmentType::TaskPrioritization => write!(f, "Task Prioritization"),
            AdjustmentType::ToolSelection => write!(f, "Tool Selection"),
            AdjustmentType::ApproachModification => write!(f, "Approach Modification"),
            AdjustmentType::ResourceReallocation => write!(f, "Resource Reallocation"),
            AdjustmentType::TimelineAdjustment => write!(f, "Timeline Adjustment"),
            AdjustmentType::QualityStandards => write!(f, "Quality Standards"),
            AdjustmentType::RiskManagement => write!(f, "Risk Management"),
            AdjustmentType::StrategyOptimization => write!(f, "Strategy Optimization"),
        }
    }
}

/// Learning insight gained from reflection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningInsight {
    pub insight_id: String,
    pub insight_type: InsightType,
    pub description: String,
    pub evidence: Vec<String>,
    pub confidence: f64,
    pub applicability: Applicability,
    pub retention_value: f64, // How valuable this insight is for future use
}

/// Types of insights
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InsightType {
    CausalRelationship,
    PerformancePattern,
    SuccessFactors,
    FailureFactors,
    EnvironmentalInfluence,
    ToolEffectiveness,
    StrategyOptimization,
}

/// Recommendation from reflection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub recommendation_id: String,
    pub recommendation_type: RecommendationType,
    pub description: String,
    pub priority: Priority,
    pub urgency: Urgency,
    pub implementation_timeline: Duration,
    pub expected_benefits: Vec<String>,
    pub potential_risks: Vec<String>,
}

/// Types of recommendations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationType {
    ImmediateAction,
    StrategyChange,
    SkillDevelopment,
    ToolAdoption,
    ProcessImprovement,
    GoalAdjustment,
    ResourceRequest,
}

/// Learning experience for building knowledge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningExperience {
    pub experience_id: String,
    pub timestamp: SystemTime,
    pub context_summary: String,
    pub actions_taken: Vec<String>,
    pub outcomes: Vec<String>,
    pub success_level: f64,
    pub lessons_learned: Vec<String>,
    pub applicability_tags: Vec<String>,
}

/// Strategy pattern for recognizing effective approaches
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyPattern {
    pub pattern_id: String,
    pub pattern_name: String,
    pub context_conditions: Vec<String>,
    pub strategy_elements: Vec<String>,
    pub success_rate: f64,
    pub usage_count: u32,
    pub last_used: SystemTime,
    pub effectiveness_trend: EffectivenessTrend,
}

/// Performance metrics for tracking improvement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub goal_completion_rate: f64,
    pub average_completion_time: Duration,
    pub error_rate: f64,
    pub efficiency_score: f64,
    pub learning_velocity: f64,
    pub strategy_adaptation_rate: f64,
}

// Supporting enums and types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VelocityTrend { Increasing, Stable, Decreasing, Volatile }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ImpactLevel { Low, Medium, High, Critical }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DifficultyLevel { Easy, Medium, Hard, Expert }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Priority { Low, Medium, High, Critical }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Urgency { Low, Medium, High, Immediate }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EffectivenessTrend { Improving, Stable, Declining }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MilestoneAchievement {
    pub milestone_name: String,
    pub achieved: bool,
    pub achievement_time: Option<SystemTime>,
    pub quality_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetrics {
    pub accuracy: f64,
    pub completeness: f64,
    pub efficiency: f64,
    pub maintainability: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bottleneck {
    pub bottleneck_id: String,
    pub description: String,
    pub severity: ImpactLevel,
    pub frequency: f64,
    pub suggested_solutions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessPattern {
    pub pattern_id: String,
    pub description: String,
    pub conditions: Vec<String>,
    pub actions: Vec<String>,
    pub success_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailurePattern {
    pub pattern_id: String,
    pub description: String,
    pub conditions: Vec<String>,
    pub actions: Vec<String>,
    pub failure_rate: f64,
    pub mitigation_strategies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUtilization {
    pub time_efficiency: f64,
    pub tool_effectiveness: HashMap<String, f64>,
    pub cognitive_load: f64,
    pub resource_waste: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Applicability {
    pub context_types: Vec<String>,
    pub goal_types: Vec<String>,
    pub confidence_level: f64,
    pub generalizability: f64,
}

impl Default for ReflectionConfig {
    fn default() -> Self {
        Self {
            reflection_frequency: 5,
            deep_reflection_frequency: 20,
            learning_retention_days: 30,
            confidence_threshold: 0.6,
            performance_threshold: 0.7,
            enable_meta_reflection: true,
            strategy_adjustment_sensitivity: 0.7,
        }
    }
}

impl ReflectionEngine {
    /// Create a new reflection engine with default configuration
    pub fn new() -> Self {
        Self {
            reflection_config: ReflectionConfig::default(),
            learning_history: Vec::new(),
            strategy_patterns: Vec::new(),
            performance_metrics: PerformanceMetrics::default(),
        }
    }

    /// Create a new reflection engine with custom configuration
    pub fn with_config(config: ReflectionConfig) -> Self {
        Self {
            reflection_config: config,
            learning_history: Vec::new(),
            strategy_patterns: Vec::new(),
            performance_metrics: PerformanceMetrics::default(),
        }
    }

    /// Determine if reflection should be triggered
    pub fn should_reflect(&self, context: &ExecutionContext) -> Option<ReflectionTrigger> {
        // Check for scheduled reflection (but not at iteration 0)
        if context.iteration_count() > 0 && context.iteration_count() % self.reflection_config.reflection_frequency == 0 {
            return Some(ReflectionTrigger::ScheduledInterval);
        }

        // Check for low confidence
        if let Some(latest_observation) = context.get_latest_observation() {
            if latest_observation.relevance_score < self.reflection_config.confidence_threshold {
                return Some(ReflectionTrigger::LowConfidence(latest_observation.relevance_score));
            }
        }

        // Check for repeated failures
        let recent_failures = self.count_recent_failures(context);
        if recent_failures >= 3 {
            return Some(ReflectionTrigger::RepeatedFailures(recent_failures));
        }

        // Check for goal stagnation
        if self.is_goal_stagnant(context) {
            return Some(ReflectionTrigger::GoalStagnation);
        }

        None
    }

    /// Perform comprehensive reflection on current execution context
    pub async fn reflect(
        &mut self,
        context: &ExecutionContext,
        reasoning_engine: &dyn ReasoningEngine,
        trigger: ReflectionTrigger,
    ) -> Result<ReflectionResult> {
        let reflection_type = self.determine_reflection_type(&trigger, context);

        // Perform the appropriate type of reflection
        let analysis = match reflection_type {
            ReflectionType::Routine => self.perform_routine_reflection(context, reasoning_engine).await?,
            ReflectionType::Triggered => self.perform_triggered_reflection(context, reasoning_engine, &trigger).await?,
            ReflectionType::Deep => self.perform_deep_reflection(context, reasoning_engine).await?,
            ReflectionType::Meta => self.perform_meta_reflection(context, reasoning_engine).await?,
            ReflectionType::Crisis => self.perform_crisis_reflection(context, reasoning_engine, &trigger).await?,
        };

        // Generate strategy adjustments based on analysis
        let strategy_adjustments = self.generate_strategy_adjustments(&analysis, context).await?;

        // Extract learning insights
        let learning_insights = self.extract_learning_insights(&analysis, context).await?;

        // Generate recommendations
        let recommendations = self.generate_recommendations(&analysis, &strategy_adjustments).await?;

        // Calculate confidence and performance assessments
        let confidence_assessment = self.calculate_confidence_assessment(&analysis);
        let performance_assessment = self.calculate_performance_assessment(&analysis);

        let reflection_result = ReflectionResult {
            reflection_id: uuid::Uuid::new_v4().to_string(),
            timestamp: SystemTime::now(),
            reflection_type,
            trigger_reason: trigger,
            analysis,
            strategy_adjustments,
            learning_insights,
            confidence_assessment,
            performance_assessment,
            recommendations,
        };

        // Store learning experience
        self.store_learning_experience(context, &reflection_result).await?;

        // Update performance metrics
        self.update_performance_metrics(&reflection_result);

        Ok(reflection_result)
    }

    /// Count recent failures in the execution context
    fn count_recent_failures(&self, context: &ExecutionContext) -> u32 {
        context.get_recent_actions()
            .iter()
            .filter(|event| matches!(event.event_type, ExecutionEventType::TaskFailed | ExecutionEventType::ErrorOccurred))
            .count() as u32
    }

    /// Check if goal progress has stagnated
    fn is_goal_stagnant(&self, context: &ExecutionContext) -> bool {
        // Simple heuristic: if no progress in recent iterations
        let recent_events = context.get_recent_actions();
        let progress_events = recent_events
            .iter()
            .filter(|event| matches!(event.event_type, ExecutionEventType::TaskCompleted))
            .count();

        progress_events == 0 && context.iteration_count() > 10
    }

    /// Determine the type of reflection needed
    fn determine_reflection_type(&self, trigger: &ReflectionTrigger, context: &ExecutionContext) -> ReflectionType {
        match trigger {
            ReflectionTrigger::ScheduledInterval => {
                if context.iteration_count() % self.reflection_config.deep_reflection_frequency == 0 {
                    ReflectionType::Deep
                } else {
                    ReflectionType::Routine
                }
            }
            ReflectionTrigger::CriticalError(_) => ReflectionType::Crisis,
            ReflectionTrigger::RepeatedFailures(_) => ReflectionType::Crisis,
            _ => ReflectionType::Triggered,
        }
    }

    /// Calculate confidence assessment from analysis
    fn calculate_confidence_assessment(&self, analysis: &ReflectionAnalysis) -> f64 {
        let progress_weight = 0.3;
        let strategy_weight = 0.3;
        let quality_weight = 0.2;
        let bottleneck_weight = 0.2;

        let progress_score = analysis.progress_assessment.goal_completion_percentage;
        let strategy_score = analysis.strategy_effectiveness.current_strategy_score;
        let quality_score = (analysis.progress_assessment.quality_metrics.accuracy +
                           analysis.progress_assessment.quality_metrics.completeness +
                           analysis.progress_assessment.quality_metrics.efficiency) / 3.0;
        let bottleneck_penalty = analysis.bottlenecks_identified.len() as f64 * 0.1;

        let weighted_score = (progress_score * progress_weight) +
                           (strategy_score * strategy_weight) +
                           (quality_score * quality_weight) -
                           (bottleneck_penalty * bottleneck_weight);

        weighted_score.max(0.0).min(1.0)
    }

    /// Calculate performance assessment from analysis
    fn calculate_performance_assessment(&self, analysis: &ReflectionAnalysis) -> f64 {
        let efficiency_weight = 0.4;
        let quality_weight = 0.3;
        let velocity_weight = 0.3;

        let efficiency_score = analysis.progress_assessment.time_efficiency;
        let quality_score = (analysis.progress_assessment.quality_metrics.accuracy +
                           analysis.progress_assessment.quality_metrics.completeness) / 2.0;
        let velocity_score = match analysis.progress_assessment.velocity_trend {
            VelocityTrend::Increasing => 1.0,
            VelocityTrend::Stable => 0.7,
            VelocityTrend::Decreasing => 0.3,
            VelocityTrend::Volatile => 0.5,
        };

        (efficiency_score * efficiency_weight) +
        (quality_score * quality_weight) +
        (velocity_score * velocity_weight)
    }

    /// Update performance metrics based on reflection results
    fn update_performance_metrics(&mut self, reflection_result: &ReflectionResult) {
        self.performance_metrics.efficiency_score =
            (self.performance_metrics.efficiency_score * 0.8) +
            (reflection_result.performance_assessment * 0.2);

        self.performance_metrics.learning_velocity =
            (self.performance_metrics.learning_velocity * 0.9) +
            (reflection_result.learning_insights.len() as f64 * 0.1);
    }

    /// Perform routine reflection (standard scheduled reflection)
    async fn perform_routine_reflection(
        &self,
        context: &ExecutionContext,
        _reasoning_engine: &dyn ReasoningEngine,
    ) -> Result<ReflectionAnalysis> {
        // Analyze current progress
        let progress_assessment = self.analyze_progress(context).await?;

        // Evaluate strategy effectiveness
        let strategy_effectiveness = self.evaluate_strategy_effectiveness(context).await?;

        // Identify learning opportunities
        let learning_opportunities = self.identify_learning_opportunities(context).await?;

        // Detect bottlenecks
        let bottlenecks_identified = self.detect_bottlenecks(context).await?;

        // Analyze patterns
        let success_patterns = self.analyze_success_patterns(context).await?;
        let failure_patterns = self.analyze_failure_patterns(context).await?;

        // Assess resource utilization
        let resource_utilization = self.assess_resource_utilization(context).await?;

        Ok(ReflectionAnalysis {
            progress_assessment,
            strategy_effectiveness,
            learning_opportunities,
            bottlenecks_identified,
            success_patterns,
            failure_patterns,
            resource_utilization,
        })
    }

    /// Perform triggered reflection (due to specific conditions)
    async fn perform_triggered_reflection(
        &self,
        context: &ExecutionContext,
        reasoning_engine: &dyn ReasoningEngine,
        trigger: &ReflectionTrigger,
    ) -> Result<ReflectionAnalysis> {
        // Start with routine analysis
        let mut analysis = self.perform_routine_reflection(context, reasoning_engine).await?;

        // Add trigger-specific analysis
        match trigger {
            ReflectionTrigger::LowConfidence(score) => {
                analysis.bottlenecks_identified.push(Bottleneck {
                    bottleneck_id: uuid::Uuid::new_v4().to_string(),
                    description: format!("Low confidence detected: {:.2}", score),
                    severity: ImpactLevel::High,
                    frequency: 1.0,
                    suggested_solutions: vec![
                        "Review recent actions for errors".to_string(),
                        "Seek additional information".to_string(),
                        "Consider alternative approaches".to_string(),
                    ],
                });
            }
            ReflectionTrigger::PoorPerformance(score) => {
                analysis.bottlenecks_identified.push(Bottleneck {
                    bottleneck_id: uuid::Uuid::new_v4().to_string(),
                    description: format!("Poor performance detected: {:.2}", score),
                    severity: ImpactLevel::High,
                    frequency: 1.0,
                    suggested_solutions: vec![
                        "Optimize current strategy".to_string(),
                        "Improve tool usage".to_string(),
                        "Enhance task prioritization".to_string(),
                    ],
                });
            }
            _ => {}
        }

        Ok(analysis)
    }

    /// Perform deep reflection (comprehensive analysis)
    async fn perform_deep_reflection(
        &self,
        context: &ExecutionContext,
        reasoning_engine: &dyn ReasoningEngine,
    ) -> Result<ReflectionAnalysis> {
        // Perform comprehensive analysis including historical patterns
        let mut analysis = self.perform_routine_reflection(context, reasoning_engine).await?;

        // Add deep analysis of historical patterns
        analysis.success_patterns = self.analyze_historical_success_patterns().await?;
        analysis.failure_patterns = self.analyze_historical_failure_patterns().await?;

        // Enhanced learning opportunity identification
        analysis.learning_opportunities.extend(
            self.identify_advanced_learning_opportunities(context).await?
        );

        Ok(analysis)
    }

    /// Perform meta-reflection (reflection on the reflection process)
    async fn perform_meta_reflection(
        &self,
        context: &ExecutionContext,
        reasoning_engine: &dyn ReasoningEngine,
    ) -> Result<ReflectionAnalysis> {
        // Analyze the effectiveness of previous reflections
        let mut analysis = self.perform_routine_reflection(context, reasoning_engine).await?;

        // Add meta-analysis
        analysis.learning_opportunities.push(LearningOpportunity {
            opportunity_id: uuid::Uuid::new_v4().to_string(),
            description: "Improve reflection process effectiveness".to_string(),
            potential_impact: ImpactLevel::Medium,
            learning_type: LearningType::StrategyOptimization,
            implementation_difficulty: DifficultyLevel::Medium,
            priority: Priority::Medium,
        });

        Ok(analysis)
    }

    /// Perform crisis reflection (emergency analysis)
    async fn perform_crisis_reflection(
        &self,
        context: &ExecutionContext,
        reasoning_engine: &dyn ReasoningEngine,
        trigger: &ReflectionTrigger,
    ) -> Result<ReflectionAnalysis> {
        let mut analysis = self.perform_triggered_reflection(context, reasoning_engine, trigger).await?;

        // Add crisis-specific analysis
        analysis.bottlenecks_identified.push(Bottleneck {
            bottleneck_id: uuid::Uuid::new_v4().to_string(),
            description: "Crisis situation requiring immediate attention".to_string(),
            severity: ImpactLevel::Critical,
            frequency: 1.0,
            suggested_solutions: vec![
                "Implement emergency recovery procedures".to_string(),
                "Revert to last known good state".to_string(),
                "Seek external assistance".to_string(),
            ],
        });

        Ok(analysis)
    }

    /// Analyze current progress towards goals
    async fn analyze_progress(&self, context: &ExecutionContext) -> Result<ProgressAssessment> {
        let total_tasks = context.active_tasks.len() + context.completed_tasks.len();
        let completed_tasks = context.completed_tasks.len();

        let goal_completion_percentage = if total_tasks > 0 {
            completed_tasks as f64 / total_tasks as f64
        } else {
            0.0
        };

        let velocity_trend = self.calculate_velocity_trend(context);
        let milestone_achievements = self.assess_milestone_achievements(context);
        let time_efficiency = self.calculate_time_efficiency(context);
        let quality_metrics = self.assess_quality_metrics(context);

        Ok(ProgressAssessment {
            goal_completion_percentage,
            velocity_trend,
            milestone_achievements,
            time_efficiency,
            quality_metrics,
        })
    }

    /// Evaluate the effectiveness of current strategy
    async fn evaluate_strategy_effectiveness(&self, context: &ExecutionContext) -> Result<StrategyEffectiveness> {
        let current_strategy_score = self.calculate_strategy_score(context);
        let strategy_consistency = self.calculate_strategy_consistency(context);
        let adaptation_frequency = context.strategy_adjustments.len() as f64 / context.iteration_count() as f64;
        let strategy_alignment = self.calculate_strategy_alignment(context);
        let execution_quality = self.calculate_execution_quality(context);

        Ok(StrategyEffectiveness {
            current_strategy_score,
            strategy_consistency,
            adaptation_frequency,
            strategy_alignment,
            execution_quality,
        })
    }

    /// Identify learning opportunities
    async fn identify_learning_opportunities(&self, context: &ExecutionContext) -> Result<Vec<LearningOpportunity>> {
        let mut opportunities = Vec::new();

        // Analyze failed tasks for learning opportunities
        for task in &context.completed_tasks {
            if task.success == Some(false) {
                opportunities.push(LearningOpportunity {
                    opportunity_id: uuid::Uuid::new_v4().to_string(),
                    description: format!("Learn from failed task: {}", task.description),
                    potential_impact: ImpactLevel::Medium,
                    learning_type: LearningType::ErrorPrevention,
                    implementation_difficulty: DifficultyLevel::Easy,
                    priority: Priority::High,
                });
            }
        }

        // Identify tool usage improvements
        if context.get_available_tools().len() > 0 {
            opportunities.push(LearningOpportunity {
                opportunity_id: uuid::Uuid::new_v4().to_string(),
                description: "Optimize tool usage patterns".to_string(),
                potential_impact: ImpactLevel::High,
                learning_type: LearningType::ToolUsageEnhancement,
                implementation_difficulty: DifficultyLevel::Medium,
                priority: Priority::Medium,
            });
        }

        Ok(opportunities)
    }

    /// Detect bottlenecks in execution
    async fn detect_bottlenecks(&self, context: &ExecutionContext) -> Result<Vec<Bottleneck>> {
        let mut bottlenecks = Vec::new();

        // Check for repeated failures
        let failure_count = context.completed_tasks
            .iter()
            .filter(|task| task.success == Some(false))
            .count();

        if failure_count > 2 {
            bottlenecks.push(Bottleneck {
                bottleneck_id: uuid::Uuid::new_v4().to_string(),
                description: "High failure rate in task execution".to_string(),
                severity: ImpactLevel::High,
                frequency: failure_count as f64 / context.completed_tasks.len() as f64,
                suggested_solutions: vec![
                    "Review task complexity".to_string(),
                    "Improve error handling".to_string(),
                    "Enhance validation".to_string(),
                ],
            });
        }

        // Check for stagnation
        if context.active_tasks.is_empty() && context.iteration_count() > 5 {
            bottlenecks.push(Bottleneck {
                bottleneck_id: uuid::Uuid::new_v4().to_string(),
                description: "No active tasks - potential stagnation".to_string(),
                severity: ImpactLevel::Medium,
                frequency: 1.0,
                suggested_solutions: vec![
                    "Generate new tasks".to_string(),
                    "Review goal decomposition".to_string(),
                    "Check for completion criteria".to_string(),
                ],
            });
        }

        Ok(bottlenecks)
    }

    /// Analyze success patterns
    async fn analyze_success_patterns(&self, context: &ExecutionContext) -> Result<Vec<SuccessPattern>> {
        let mut patterns = Vec::new();

        let successful_tasks: Vec<_> = context.completed_tasks
            .iter()
            .filter(|task| task.success == Some(true))
            .collect();

        if !successful_tasks.is_empty() {
            patterns.push(SuccessPattern {
                pattern_id: uuid::Uuid::new_v4().to_string(),
                description: "Successful task completion pattern".to_string(),
                conditions: vec!["Clear task definition".to_string(), "Adequate resources".to_string()],
                actions: vec!["Systematic execution".to_string(), "Regular validation".to_string()],
                success_rate: successful_tasks.len() as f64 / context.completed_tasks.len() as f64,
            });
        }

        Ok(patterns)
    }

    /// Analyze failure patterns
    async fn analyze_failure_patterns(&self, context: &ExecutionContext) -> Result<Vec<FailurePattern>> {
        let mut patterns = Vec::new();

        let failed_tasks: Vec<_> = context.completed_tasks
            .iter()
            .filter(|task| task.success == Some(false))
            .collect();

        if !failed_tasks.is_empty() {
            patterns.push(FailurePattern {
                pattern_id: uuid::Uuid::new_v4().to_string(),
                description: "Task failure pattern".to_string(),
                conditions: vec!["Unclear requirements".to_string(), "Resource constraints".to_string()],
                actions: vec!["Rushed execution".to_string(), "Insufficient validation".to_string()],
                failure_rate: failed_tasks.len() as f64 / context.completed_tasks.len() as f64,
                mitigation_strategies: vec![
                    "Improve task planning".to_string(),
                    "Enhance validation steps".to_string(),
                    "Allocate sufficient resources".to_string(),
                ],
            });
        }

        Ok(patterns)
    }

    /// Assess resource utilization
    async fn assess_resource_utilization(&self, context: &ExecutionContext) -> Result<ResourceUtilization> {
        let time_efficiency = self.calculate_time_efficiency(context);
        let mut tool_effectiveness = HashMap::new();

        // Assess tool effectiveness (simplified)
        for tool in context.get_available_tools() {
            tool_effectiveness.insert(tool.clone(), 0.8); // Default effectiveness
        }

        Ok(ResourceUtilization {
            time_efficiency,
            tool_effectiveness,
            cognitive_load: 0.7, // Placeholder
            resource_waste: 0.2, // Placeholder
        })
    }

    // Helper methods for calculations
    fn calculate_velocity_trend(&self, context: &ExecutionContext) -> VelocityTrend {
        // Simplified calculation based on recent task completion
        let recent_completions = context.completed_tasks
            .iter()
            .filter(|task| task.completed_at.is_some())
            .count();

        if recent_completions > context.iteration_count() as usize / 2 {
            VelocityTrend::Increasing
        } else if recent_completions > context.iteration_count() as usize / 4 {
            VelocityTrend::Stable
        } else {
            VelocityTrend::Decreasing
        }
    }

    fn assess_milestone_achievements(&self, context: &ExecutionContext) -> Vec<MilestoneAchievement> {
        // Simplified milestone assessment
        vec![MilestoneAchievement {
            milestone_name: "Initial progress".to_string(),
            achieved: !context.completed_tasks.is_empty(),
            achievement_time: context.completed_tasks.first().and_then(|t| t.completed_at),
            quality_score: 0.8,
        }]
    }

    fn calculate_time_efficiency(&self, context: &ExecutionContext) -> f64 {
        // Simplified efficiency calculation
        let total_time = context.get_execution_duration().as_secs() as f64;
        let completed_tasks = context.completed_tasks.len() as f64;

        if total_time > 0.0 && completed_tasks > 0.0 {
            (completed_tasks / total_time * 3600.0).min(1.0) // Tasks per hour, capped at 1.0
        } else {
            0.5
        }
    }

    fn assess_quality_metrics(&self, context: &ExecutionContext) -> QualityMetrics {
        let successful_tasks = context.completed_tasks
            .iter()
            .filter(|task| task.success == Some(true))
            .count() as f64;
        let total_tasks = context.completed_tasks.len() as f64;

        let accuracy = if total_tasks > 0.0 { successful_tasks / total_tasks } else { 1.0 };

        QualityMetrics {
            accuracy,
            completeness: 0.8, // Placeholder
            efficiency: self.calculate_time_efficiency(context),
            maintainability: 0.7, // Placeholder
        }
    }

    fn calculate_strategy_score(&self, context: &ExecutionContext) -> f64 {
        // Simplified strategy scoring based on success rate
        let successful_tasks = context.completed_tasks
            .iter()
            .filter(|task| task.success == Some(true))
            .count() as f64;
        let total_tasks = context.completed_tasks.len() as f64;

        if total_tasks > 0.0 {
            successful_tasks / total_tasks
        } else {
            0.5
        }
    }

    fn calculate_strategy_consistency(&self, context: &ExecutionContext) -> f64 {
        // Simplified consistency calculation
        let adjustment_frequency = context.strategy_adjustments.len() as f64 / context.iteration_count() as f64;
        (1.0 - adjustment_frequency).max(0.0)
    }

    fn calculate_strategy_alignment(&self, context: &ExecutionContext) -> f64 {
        // Simplified alignment calculation
        if context.current_goal.is_some() && !context.active_tasks.is_empty() {
            0.8
        } else {
            0.4
        }
    }

    fn calculate_execution_quality(&self, context: &ExecutionContext) -> f64 {
        self.assess_quality_metrics(context).accuracy
    }

    /// Generate strategy adjustments based on analysis
    async fn generate_strategy_adjustments(
        &self,
        analysis: &ReflectionAnalysis,
        _context: &ExecutionContext,
    ) -> Result<Vec<StrategyAdjustment>> {
        let mut adjustments = Vec::new();

        // Generate adjustments based on bottlenecks
        for bottleneck in &analysis.bottlenecks_identified {
            if bottleneck.severity == ImpactLevel::High || bottleneck.severity == ImpactLevel::Critical {
                adjustments.push(StrategyAdjustment {
                    adjustment_id: uuid::Uuid::new_v4().to_string(),
                    adjustment_type: AdjustmentType::ApproachModification,
                    description: format!("Address bottleneck: {}", bottleneck.description),
                    rationale: "High-impact bottleneck requires strategy adjustment".to_string(),
                    expected_impact: bottleneck.severity.clone(),
                    implementation_steps: bottleneck.suggested_solutions.clone(),
                    success_metrics: vec!["Reduced failure rate".to_string(), "Improved efficiency".to_string()],
                    rollback_plan: Some("Revert to previous approach if no improvement".to_string()),
                });
            }
        }

        // Generate adjustments based on poor performance
        if analysis.strategy_effectiveness.current_strategy_score < self.reflection_config.performance_threshold {
            adjustments.push(StrategyAdjustment {
                adjustment_id: uuid::Uuid::new_v4().to_string(),
                adjustment_type: AdjustmentType::StrategyOptimization,
                description: "Optimize overall strategy due to poor performance".to_string(),
                rationale: format!("Strategy score {:.2} below threshold {:.2}",
                                 analysis.strategy_effectiveness.current_strategy_score,
                                 self.reflection_config.performance_threshold),
                expected_impact: ImpactLevel::High,
                implementation_steps: vec![
                    "Review current approach".to_string(),
                    "Identify alternative strategies".to_string(),
                    "Implement gradual changes".to_string(),
                ],
                success_metrics: vec!["Improved strategy score".to_string(), "Better goal progress".to_string()],
                rollback_plan: Some("Return to baseline strategy".to_string()),
            });
        }

        Ok(adjustments)
    }

    /// Extract learning insights from analysis
    async fn extract_learning_insights(
        &self,
        analysis: &ReflectionAnalysis,
        _context: &ExecutionContext,
    ) -> Result<Vec<LearningInsight>> {
        let mut insights = Vec::new();

        // Extract insights from success patterns
        for pattern in &analysis.success_patterns {
            insights.push(LearningInsight {
                insight_id: uuid::Uuid::new_v4().to_string(),
                insight_type: InsightType::SuccessFactors,
                description: format!("Success pattern identified: {}", pattern.description),
                evidence: pattern.conditions.clone(),
                confidence: pattern.success_rate,
                applicability: Applicability {
                    context_types: vec!["similar_goals".to_string()],
                    goal_types: vec!["analysis".to_string(), "problem_solving".to_string()],
                    confidence_level: pattern.success_rate,
                    generalizability: 0.7,
                },
                retention_value: pattern.success_rate,
            });
        }

        // Extract insights from failure patterns
        for pattern in &analysis.failure_patterns {
            insights.push(LearningInsight {
                insight_id: uuid::Uuid::new_v4().to_string(),
                insight_type: InsightType::FailureFactors,
                description: format!("Failure pattern identified: {}", pattern.description),
                evidence: pattern.conditions.clone(),
                confidence: pattern.failure_rate,
                applicability: Applicability {
                    context_types: vec!["similar_goals".to_string()],
                    goal_types: vec!["analysis".to_string(), "problem_solving".to_string()],
                    confidence_level: pattern.failure_rate,
                    generalizability: 0.6,
                },
                retention_value: pattern.failure_rate * 0.8, // Slightly lower retention for failure patterns
            });
        }

        Ok(insights)
    }

    /// Generate recommendations based on analysis and adjustments
    async fn generate_recommendations(
        &self,
        analysis: &ReflectionAnalysis,
        strategy_adjustments: &[StrategyAdjustment],
    ) -> Result<Vec<Recommendation>> {
        let mut recommendations = Vec::new();

        // Generate immediate action recommendations
        for adjustment in strategy_adjustments {
            if adjustment.expected_impact == ImpactLevel::Critical {
                recommendations.push(Recommendation {
                    recommendation_id: uuid::Uuid::new_v4().to_string(),
                    recommendation_type: RecommendationType::ImmediateAction,
                    description: format!("Implement critical adjustment: {}", adjustment.description),
                    priority: Priority::Critical,
                    urgency: Urgency::Immediate,
                    implementation_timeline: Duration::from_secs(300), // 5 minutes
                    expected_benefits: vec!["Prevent further degradation".to_string()],
                    potential_risks: vec!["Disruption to current workflow".to_string()],
                });
            }
        }

        // Generate learning recommendations
        for opportunity in &analysis.learning_opportunities {
            if opportunity.potential_impact == ImpactLevel::High {
                recommendations.push(Recommendation {
                    recommendation_id: uuid::Uuid::new_v4().to_string(),
                    recommendation_type: RecommendationType::SkillDevelopment,
                    description: format!("Pursue learning opportunity: {}", opportunity.description),
                    priority: opportunity.priority.clone(),
                    urgency: Urgency::Medium,
                    implementation_timeline: Duration::from_secs(3600), // 1 hour
                    expected_benefits: vec!["Improved capabilities".to_string(), "Better performance".to_string()],
                    potential_risks: vec!["Time investment required".to_string()],
                });
            }
        }

        Ok(recommendations)
    }

    /// Store learning experience for future reference
    async fn store_learning_experience(
        &mut self,
        context: &ExecutionContext,
        reflection_result: &ReflectionResult,
    ) -> Result<()> {
        let experience = LearningExperience {
            experience_id: uuid::Uuid::new_v4().to_string(),
            timestamp: SystemTime::now(),
            context_summary: context.get_summary(),
            actions_taken: context.get_action_history(),
            outcomes: vec![reflection_result.analysis.progress_assessment.goal_completion_percentage.to_string()],
            success_level: reflection_result.performance_assessment,
            lessons_learned: reflection_result.learning_insights
                .iter()
                .map(|insight| insight.description.clone())
                .collect(),
            applicability_tags: vec!["general".to_string()],
        };

        self.learning_history.push(experience);

        // Keep only recent experiences (based on retention days)
        let cutoff_time = SystemTime::now() - Duration::from_secs(
            self.reflection_config.learning_retention_days as u64 * 24 * 60 * 60
        );
        self.learning_history.retain(|exp| exp.timestamp > cutoff_time);

        Ok(())
    }

    /// Analyze historical success patterns
    async fn analyze_historical_success_patterns(&self) -> Result<Vec<SuccessPattern>> {
        let mut patterns = Vec::new();

        let successful_experiences: Vec<_> = self.learning_history
            .iter()
            .filter(|exp| exp.success_level > 0.7)
            .collect();

        if !successful_experiences.is_empty() {
            patterns.push(SuccessPattern {
                pattern_id: uuid::Uuid::new_v4().to_string(),
                description: "Historical success pattern".to_string(),
                conditions: vec!["High confidence".to_string(), "Clear objectives".to_string()],
                actions: vec!["Systematic approach".to_string(), "Regular validation".to_string()],
                success_rate: successful_experiences.len() as f64 / self.learning_history.len() as f64,
            });
        }

        Ok(patterns)
    }

    /// Analyze historical failure patterns
    async fn analyze_historical_failure_patterns(&self) -> Result<Vec<FailurePattern>> {
        let mut patterns = Vec::new();

        let failed_experiences: Vec<_> = self.learning_history
            .iter()
            .filter(|exp| exp.success_level < 0.3)
            .collect();

        if !failed_experiences.is_empty() {
            patterns.push(FailurePattern {
                pattern_id: uuid::Uuid::new_v4().to_string(),
                description: "Historical failure pattern".to_string(),
                conditions: vec!["Low confidence".to_string(), "Unclear objectives".to_string()],
                actions: vec!["Rushed approach".to_string(), "Insufficient validation".to_string()],
                failure_rate: failed_experiences.len() as f64 / self.learning_history.len() as f64,
                mitigation_strategies: vec![
                    "Improve planning".to_string(),
                    "Enhance validation".to_string(),
                    "Seek clarification".to_string(),
                ],
            });
        }

        Ok(patterns)
    }

    /// Identify advanced learning opportunities
    async fn identify_advanced_learning_opportunities(&self, _context: &ExecutionContext) -> Result<Vec<LearningOpportunity>> {
        let mut opportunities = Vec::new();

        // Analyze patterns across learning history
        if self.learning_history.len() > 5 {
            opportunities.push(LearningOpportunity {
                opportunity_id: uuid::Uuid::new_v4().to_string(),
                description: "Pattern recognition across historical experiences".to_string(),
                potential_impact: ImpactLevel::High,
                learning_type: LearningType::PatternRecognition,
                implementation_difficulty: DifficultyLevel::Hard,
                priority: Priority::Medium,
            });
        }

        Ok(opportunities)
    }

    /// Get reflection statistics
    pub fn get_reflection_statistics(&self) -> ReflectionStatistics {
        ReflectionStatistics {
            total_learning_experiences: self.learning_history.len(),
            total_strategy_patterns: self.strategy_patterns.len(),
            average_success_rate: self.calculate_average_success_rate(),
            learning_velocity: self.performance_metrics.learning_velocity,
            reflection_frequency: self.reflection_config.reflection_frequency,
        }
    }

    fn calculate_average_success_rate(&self) -> f64 {
        if self.learning_history.is_empty() {
            return 0.0;
        }

        let total_success: f64 = self.learning_history
            .iter()
            .map(|exp| exp.success_level)
            .sum();

        total_success / self.learning_history.len() as f64
    }
}

/// Statistics about the reflection system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionStatistics {
    pub total_learning_experiences: usize,
    pub total_strategy_patterns: usize,
    pub average_success_rate: f64,
    pub learning_velocity: f64,
    pub reflection_frequency: u32,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            goal_completion_rate: 0.0,
            average_completion_time: Duration::from_secs(0),
            error_rate: 0.0,
            efficiency_score: 0.5,
            learning_velocity: 0.0,
            strategy_adaptation_rate: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::goal::{Goal, GoalType, GoalPriority};
    use std::collections::HashMap;

    #[test]
    fn test_reflection_engine_creation() {
        let engine = ReflectionEngine::new();
        assert_eq!(engine.reflection_config.reflection_frequency, 5);
        assert_eq!(engine.learning_history.len(), 0);
        assert_eq!(engine.strategy_patterns.len(), 0);
    }

    #[test]
    fn test_reflection_config_default() {
        let config = ReflectionConfig::default();
        assert_eq!(config.reflection_frequency, 5);
        assert_eq!(config.deep_reflection_frequency, 20);
        assert_eq!(config.confidence_threshold, 0.6);
        assert_eq!(config.performance_threshold, 0.7);
        assert!(config.enable_meta_reflection);
    }

    #[test]
    fn test_should_reflect_scheduled() {
        let engine = ReflectionEngine::new();

        // Create a test context
        let goal = Goal {
            goal_id: "test-goal".to_string(),
            description: "Test goal".to_string(),
            goal_type: GoalType::Analysis,
            priority: GoalPriority::Medium,
            success_criteria: Vec::new(),
            max_iterations: None,
            timeout: None,
            metadata: HashMap::new(),
        };

        let mut context = ExecutionContext::new(goal);

        // Should not reflect at iteration 0
        assert!(engine.should_reflect(&context).is_none());

        // Should reflect at iteration 5 (default frequency)
        for _ in 1..=5 {
            context.increment_iteration();
        }

        // Now context should be at iteration 5
        assert_eq!(context.iteration_count(), 5);

        let trigger = engine.should_reflect(&context);
        assert!(trigger.is_some());
        assert!(matches!(trigger.unwrap(), ReflectionTrigger::ScheduledInterval));
    }

    #[test]
    fn test_reflection_type_determination() {
        let engine = ReflectionEngine::new();

        let goal = Goal {
            goal_id: "test-goal".to_string(),
            description: "Test goal".to_string(),
            goal_type: GoalType::Analysis,
            priority: GoalPriority::Medium,
            success_criteria: Vec::new(),
            max_iterations: None,
            timeout: None,
            metadata: HashMap::new(),
        };

        let mut context = ExecutionContext::new(goal);

        // Set iteration count to 5 for routine reflection
        for _ in 1..=5 {
            context.increment_iteration();
        }

        // Test different trigger types
        let routine_type = engine.determine_reflection_type(
            &ReflectionTrigger::ScheduledInterval,
            &context
        );
        assert!(matches!(routine_type, ReflectionType::Routine));

        let crisis_type = engine.determine_reflection_type(
            &ReflectionTrigger::CriticalError("test error".to_string()),
            &context
        );
        assert!(matches!(crisis_type, ReflectionType::Crisis));

        let triggered_type = engine.determine_reflection_type(
            &ReflectionTrigger::LowConfidence(0.3),
            &context
        );
        assert!(matches!(triggered_type, ReflectionType::Triggered));
    }

    #[test]
    fn test_count_recent_failures() {
        let engine = ReflectionEngine::new();

        let goal = Goal {
            goal_id: "test-goal".to_string(),
            description: "Test goal".to_string(),
            goal_type: GoalType::Analysis,
            priority: GoalPriority::Medium,
            success_criteria: Vec::new(),
            max_iterations: None,
            timeout: None,
            metadata: HashMap::new(),
        };

        let mut context = ExecutionContext::new(goal);

        // Add some failure events
        context.execution_history.push(crate::context::ExecutionEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            timestamp: SystemTime::now(),
            event_type: ExecutionEventType::TaskFailed,
            description: "Task failed".to_string(),
            metadata: HashMap::new(),
        });

        context.execution_history.push(crate::context::ExecutionEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            timestamp: SystemTime::now(),
            event_type: ExecutionEventType::ErrorOccurred,
            description: "Error occurred".to_string(),
            metadata: HashMap::new(),
        });

        let failure_count = engine.count_recent_failures(&context);
        assert_eq!(failure_count, 2);
    }

    #[test]
    fn test_calculate_confidence_assessment() {
        let engine = ReflectionEngine::new();

        let analysis = ReflectionAnalysis {
            progress_assessment: ProgressAssessment {
                goal_completion_percentage: 0.9,
                velocity_trend: VelocityTrend::Increasing,
                milestone_achievements: Vec::new(),
                time_efficiency: 0.9,
                quality_metrics: QualityMetrics {
                    accuracy: 0.95,
                    completeness: 0.9,
                    efficiency: 0.9,
                    maintainability: 0.8,
                },
            },
            strategy_effectiveness: StrategyEffectiveness {
                current_strategy_score: 0.9,
                strategy_consistency: 0.8,
                adaptation_frequency: 0.1,
                strategy_alignment: 0.9,
                execution_quality: 0.9,
            },
            learning_opportunities: Vec::new(),
            bottlenecks_identified: Vec::new(), // No bottlenecks for high confidence
            success_patterns: Vec::new(),
            failure_patterns: Vec::new(),
            resource_utilization: ResourceUtilization {
                time_efficiency: 0.9,
                tool_effectiveness: HashMap::new(),
                cognitive_load: 0.5,
                resource_waste: 0.1,
            },
        };

        let confidence = engine.calculate_confidence_assessment(&analysis);
        assert!(confidence > 0.6); // Adjusted threshold
        assert!(confidence <= 1.0);
    }

    #[test]
    fn test_performance_metrics_default() {
        let metrics = PerformanceMetrics::default();
        assert_eq!(metrics.goal_completion_rate, 0.0);
        assert_eq!(metrics.efficiency_score, 0.5);
        assert_eq!(metrics.learning_velocity, 0.0);
    }

    #[test]
    fn test_reflection_statistics() {
        let mut engine = ReflectionEngine::new();

        // Add some learning experiences
        engine.learning_history.push(LearningExperience {
            experience_id: uuid::Uuid::new_v4().to_string(),
            timestamp: SystemTime::now(),
            context_summary: "Test context".to_string(),
            actions_taken: vec!["action1".to_string()],
            outcomes: vec!["outcome1".to_string()],
            success_level: 0.8,
            lessons_learned: vec!["lesson1".to_string()],
            applicability_tags: vec!["tag1".to_string()],
        });

        let stats = engine.get_reflection_statistics();
        assert_eq!(stats.total_learning_experiences, 1);
        assert_eq!(stats.total_strategy_patterns, 0);
        assert_eq!(stats.average_success_rate, 0.8);
        assert_eq!(stats.reflection_frequency, 5);
    }
}
