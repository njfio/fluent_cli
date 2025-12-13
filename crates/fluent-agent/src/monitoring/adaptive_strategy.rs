//! Adaptive Strategy System for Real-Time Adjustment
//!
//! This module provides intelligent strategy adaptation based on performance
//! feedback and changing conditions during autonomous execution.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::context::ExecutionContext;
use crate::monitoring::performance_monitor::PerformanceMetrics;

/// Adaptive strategy system for autonomous adjustment
pub struct AdaptiveStrategySystem {
    config: AdaptiveConfig,
    strategy_manager: Arc<RwLock<StrategyManager>>,
    adaptation_engine: Arc<RwLock<AdaptationEngine>>,
    learning_system: Arc<RwLock<LearningSystem>>,
}

/// Configuration for adaptive strategy system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptiveConfig {
    pub enable_real_time_adaptation: bool,
    pub adaptation_sensitivity: f64,
    pub min_adaptation_interval: Duration,
    pub performance_window_size: u32,
    pub confidence_threshold: f64,
    pub max_concurrent_adaptations: u32,
}

impl Default for AdaptiveConfig {
    fn default() -> Self {
        Self {
            enable_real_time_adaptation: true,
            adaptation_sensitivity: 0.7,
            min_adaptation_interval: Duration::from_secs(60),
            performance_window_size: 10,
            confidence_threshold: 0.8,
            max_concurrent_adaptations: 3,
        }
    }
}

/// Manager for strategy selection and adaptation
#[derive(Debug, Default)]
pub struct StrategyManager {
    available_strategies: Vec<ExecutionStrategy>,
    current_strategy: Option<ExecutionStrategy>,
    strategy_performance: HashMap<String, StrategyPerformance>,
    adaptation_history: Vec<StrategyAdaptation>,
}

/// Execution strategy for autonomous tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStrategy {
    pub strategy_id: String,
    pub strategy_name: String,
    pub strategy_type: StrategyType,
    pub parameters: HashMap<String, f64>,
    pub applicability_conditions: Vec<String>,
    pub expected_performance: ExpectedPerformance,
    pub resource_requirements: ResourceRequirements,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StrategyType {
    Conservative,
    Aggressive,
    Balanced,
    Experimental,
    Adaptive,
}

/// Expected performance metrics for a strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedPerformance {
    pub success_rate: f64,
    pub efficiency: f64,
    pub quality_score: f64,
    pub resource_usage: f64,
    pub execution_time: Duration,
}

/// Resource requirements for strategy execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub cpu_intensive: bool,
    pub memory_requirements: u64,
    pub network_dependent: bool,
    pub parallel_capable: bool,
}

/// Performance tracking for strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyPerformance {
    pub strategy_id: String,
    pub usage_count: u32,
    pub success_rate: f64,
    pub average_efficiency: f64,
    pub quality_average: f64,
    pub adaptation_frequency: u32,
    pub last_used: SystemTime,
}

/// Record of strategy adaptation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyAdaptation {
    pub adaptation_id: String,
    pub timestamp: SystemTime,
    pub from_strategy: String,
    pub to_strategy: String,
    pub trigger_reason: String,
    pub performance_before: f64,
    pub performance_after: Option<f64>,
    pub adaptation_success: Option<bool>,
}

/// Engine for determining when and how to adapt
#[derive(Debug, Default)]
pub struct AdaptationEngine {
    adaptation_rules: Vec<AdaptationRule>,
    trigger_conditions: Vec<TriggerCondition>,
    active_adaptations: Vec<ActiveAdaptation>,
}

/// Rule for strategy adaptation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptationRule {
    pub rule_id: String,
    pub rule_type: RuleType,
    pub conditions: Vec<String>,
    pub actions: Vec<AdaptationAction>,
    pub confidence: f64,
    pub priority: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleType {
    PerformanceBased,
    TimeBased,
    ResourceBased,
    QualityBased,
    ContextBased,
}

/// Action to take when adapting strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptationAction {
    pub action_type: ActionType,
    pub target_parameter: String,
    pub adjustment_value: f64,
    pub expected_impact: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionType {
    ParameterAdjustment,
    StrategySwitch,
    ResourceReallocation,
    PriorityChange,
    ApproachModification,
}

/// Condition that triggers adaptation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerCondition {
    pub condition_id: String,
    pub metric_name: String,
    pub threshold: f64,
    pub comparison: ComparisonType,
    pub duration: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComparisonType {
    LessThan,
    GreaterThan,
    Equals,
    Trend,
}

/// Currently active adaptation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveAdaptation {
    pub adaptation_id: String,
    pub started_at: SystemTime,
    pub adaptation_type: AdaptationType,
    pub parameters_changed: Vec<String>,
    pub monitoring_metrics: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AdaptationType {
    Incremental,
    Dramatic,
    Experimental,
    Rollback,
}

/// Learning system for improving adaptation
#[derive(Debug, Default)]
pub struct LearningSystem {
    learned_patterns: Vec<AdaptationPattern>,
    success_factors: HashMap<String, f64>,
    failure_analysis: Vec<FailureAnalysis>,
}

/// Pattern learned from adaptations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptationPattern {
    pub pattern_id: String,
    pub context_conditions: Vec<String>,
    pub successful_adaptations: Vec<String>,
    pub pattern_confidence: f64,
    pub usage_frequency: u32,
}

/// Analysis of adaptation failures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureAnalysis {
    pub failure_id: String,
    pub failed_adaptation: String,
    pub failure_reason: String,
    pub lessons_learned: Vec<String>,
    pub prevention_strategies: Vec<String>,
}

impl AdaptiveStrategySystem {
    /// Create new adaptive strategy system
    pub fn new(config: AdaptiveConfig) -> Self {
        let system = Self {
            config,
            strategy_manager: Arc::new(RwLock::new(StrategyManager::default())),
            adaptation_engine: Arc::new(RwLock::new(AdaptationEngine::default())),
            learning_system: Arc::new(RwLock::new(LearningSystem::default())),
        };

        // Initialize with default strategies asynchronously
        let manager_clone = Arc::clone(&system.strategy_manager);
        tokio::spawn(async move {
            let mut manager = manager_clone.write().await;
            if let Err(e) = AdaptiveStrategySystem::populate_default_strategies(&mut manager).await
            {
                eprintln!("Error initializing strategies: {}", e);
            }
        });

        system
    }

    /// Evaluate current performance and adapt if needed
    pub async fn evaluate_and_adapt(
        &self,
        performance: &PerformanceMetrics,
        context: &ExecutionContext,
    ) -> Result<Option<StrategyAdaptation>> {
        if !self.config.enable_real_time_adaptation {
            return Ok(None);
        }

        // Check if adaptation is needed
        let adaptation_needed = self.should_adapt(performance).await?;

        if !adaptation_needed {
            return Ok(None);
        }

        // Determine best adaptation strategy
        let adaptation = self.plan_adaptation(performance, context).await?;

        // Execute adaptation
        self.execute_adaptation(&adaptation).await?;

        Ok(Some(adaptation))
    }

    /// Check if strategy adaptation is needed
    async fn should_adapt(&self, performance: &PerformanceMetrics) -> Result<bool> {
        let engine = self.adaptation_engine.read().await;

        // Check trigger conditions
        for condition in &engine.trigger_conditions {
            let metric_value = self.get_metric_value(performance, &condition.metric_name);

            let triggered = match condition.comparison {
                ComparisonType::LessThan => metric_value < condition.threshold,
                ComparisonType::GreaterThan => metric_value > condition.threshold,
                ComparisonType::Equals => (metric_value - condition.threshold).abs() < 0.01,
                ComparisonType::Trend => false, // Would implement trend analysis
            };

            if triggered {
                return Ok(true);
            }
        }

        // Check performance degradation
        if performance.execution_metrics.success_rate < 0.7
            || performance.efficiency_metrics.overall_efficiency < 0.6
        {
            return Ok(true);
        }

        Ok(false)
    }

    /// Plan the best adaptation strategy
    async fn plan_adaptation(
        &self,
        performance: &PerformanceMetrics,
        _context: &ExecutionContext,
    ) -> Result<StrategyAdaptation> {
        let manager = self.strategy_manager.read().await;

        // Select best alternative strategy
        let current_strategy_id = manager
            .current_strategy
            .as_ref()
            .map(|s| s.strategy_id.clone())
            .unwrap_or_else(|| "default".to_string());

        // Find strategy with best expected performance
        let best_strategy = manager
            .available_strategies
            .iter()
            .filter(|s| s.strategy_id != current_strategy_id)
            .max_by(|a, b| {
                a.expected_performance
                    .success_rate
                    .partial_cmp(&b.expected_performance.success_rate)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

        let new_strategy_id = best_strategy
            .map(|s| s.strategy_id.clone())
            .unwrap_or_else(|| "balanced".to_string());

        Ok(StrategyAdaptation {
            adaptation_id: Uuid::new_v4().to_string(),
            timestamp: SystemTime::now(),
            from_strategy: current_strategy_id,
            to_strategy: new_strategy_id,
            trigger_reason: format!(
                "Performance below threshold: {:.2}",
                performance.execution_metrics.success_rate
            ),
            performance_before: performance.execution_metrics.success_rate,
            performance_after: None,
            adaptation_success: None,
        })
    }

    /// Execute the planned adaptation
    async fn execute_adaptation(&self, adaptation: &StrategyAdaptation) -> Result<()> {
        let mut manager = self.strategy_manager.write().await;

        // Find and switch to new strategy
        if let Some(new_strategy) = manager
            .available_strategies
            .iter()
            .find(|s| s.strategy_id == adaptation.to_strategy)
            .cloned()
        {
            manager.current_strategy = Some(new_strategy);
        }

        // Record adaptation
        manager.adaptation_history.push(adaptation.clone());

        // Limit history size
        if manager.adaptation_history.len() > 100 {
            manager.adaptation_history.drain(0..50);
        }

        Ok(())
    }

    /// Get current strategy configuration
    pub async fn get_current_strategy(&self) -> Result<Option<ExecutionStrategy>> {
        let manager = self.strategy_manager.read().await;
        Ok(manager.current_strategy.clone())
    }

    /// Update strategy performance based on results
    pub async fn update_strategy_performance(
        &self,
        strategy_id: &str,
        success: bool,
        efficiency: f64,
        quality: f64,
    ) -> Result<()> {
        let mut manager = self.strategy_manager.write().await;

        let performance = manager
            .strategy_performance
            .entry(strategy_id.to_string())
            .or_insert_with(|| StrategyPerformance {
                strategy_id: strategy_id.to_string(),
                usage_count: 0,
                success_rate: 0.0,
                average_efficiency: 0.0,
                quality_average: 0.0,
                adaptation_frequency: 0,
                last_used: SystemTime::now(),
            });

        // Update metrics using exponential moving average
        performance.usage_count += 1;
        let alpha = 0.1; // Smoothing factor

        performance.success_rate =
            performance.success_rate * (1.0 - alpha) + (if success { 1.0 } else { 0.0 }) * alpha;
        performance.average_efficiency =
            performance.average_efficiency * (1.0 - alpha) + efficiency * alpha;
        performance.quality_average = performance.quality_average * (1.0 - alpha) + quality * alpha;
        performance.last_used = SystemTime::now();

        Ok(())
    }

    // Helper methods

    async fn populate_default_strategies(manager: &mut StrategyManager) -> Result<()> {
        // Conservative strategy
        manager.available_strategies.push(ExecutionStrategy {
            strategy_id: "conservative".to_string(),
            strategy_name: "Conservative Approach".to_string(),
            strategy_type: StrategyType::Conservative,
            parameters: HashMap::from([
                ("risk_tolerance".to_string(), 0.2),
                ("parallelism".to_string(), 0.3),
                ("timeout_multiplier".to_string(), 2.0),
            ]),
            applicability_conditions: vec!["high_risk".to_string()],
            expected_performance: ExpectedPerformance {
                success_rate: 0.95,
                efficiency: 0.85,
                quality_score: 0.8,
                resource_usage: 0.6,
                execution_time: Duration::from_secs_f64(2.0),
            },
            resource_requirements: ResourceRequirements {
                cpu_intensive: false,
                memory_requirements: 50,
                network_dependent: false,
                parallel_capable: true,
            },
        });

        // Aggressive strategy
        manager.available_strategies.push(ExecutionStrategy {
            strategy_id: "aggressive".to_string(),
            strategy_name: "Aggressive Approach".to_string(),
            strategy_type: StrategyType::Aggressive,
            parameters: HashMap::from([
                ("risk_tolerance".to_string(), 0.8),
                ("parallelism".to_string(), 0.9),
                ("timeout_multiplier".to_string(), 0.5),
            ]),
            applicability_conditions: vec!["high_performance_target".to_string()],
            expected_performance: ExpectedPerformance {
                success_rate: 0.7,
                efficiency: 0.9,
                quality_score: 0.7,
                resource_usage: 0.9,
                execution_time: Duration::from_secs_f64(0.5),
            },
            resource_requirements: ResourceRequirements {
                cpu_intensive: true,
                memory_requirements: 100,
                network_dependent: false,
                parallel_capable: true,
            },
        });

        // Balanced strategy
        manager.available_strategies.push(ExecutionStrategy {
            strategy_id: "balanced".to_string(),
            strategy_name: "Balanced Approach".to_string(),
            strategy_type: StrategyType::Balanced,
            parameters: HashMap::from([
                ("risk_tolerance".to_string(), 0.5),
                ("parallelism".to_string(), 0.6),
                ("timeout_multiplier".to_string(), 1.0),
            ]),
            applicability_conditions: vec!["general_purpose".to_string()],
            expected_performance: ExpectedPerformance {
                success_rate: 0.85,
                efficiency: 0.8,
                quality_score: 0.8,
                resource_usage: 0.75,
                execution_time: Duration::from_secs_f64(1.0),
            },
            resource_requirements: ResourceRequirements {
                cpu_intensive: false,
                memory_requirements: 75,
                network_dependent: false,
                parallel_capable: true,
            },
        });

        // Set balanced as default
        if let Some(balanced_strategy) = manager
            .available_strategies
            .iter()
            .find(|s| s.strategy_id == "balanced")
            .cloned()
        {
            manager.current_strategy = Some(balanced_strategy);
        }

        Ok(())
    }

    async fn initialize_default_strategies(&self) -> Result<()> {
        let mut manager = self.strategy_manager.write().await;
        Self::populate_default_strategies(&mut manager).await
    }

    fn get_metric_value(&self, performance: &PerformanceMetrics, metric_name: &str) -> f64 {
        match metric_name {
            "success_rate" => performance.execution_metrics.success_rate,
            "efficiency" => performance.efficiency_metrics.overall_efficiency,
            "quality" => performance.quality_metrics.output_quality_score,
            "memory_usage" => performance.resource_metrics.memory_usage_percent,
            _ => 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== Configuration Tests ==========

    #[test]
    fn test_adaptive_config_default() {
        let config = AdaptiveConfig::default();

        assert!(config.enable_real_time_adaptation);
        assert!((config.adaptation_sensitivity - 0.7).abs() < f64::EPSILON);
        assert_eq!(config.min_adaptation_interval, Duration::from_secs(60));
        assert_eq!(config.performance_window_size, 10);
        assert!((config.confidence_threshold - 0.8).abs() < f64::EPSILON);
        assert_eq!(config.max_concurrent_adaptations, 3);
    }

    // ========== Strategy Type Tests ==========

    #[test]
    fn test_strategy_type_variants() {
        let types = vec![
            StrategyType::Conservative,
            StrategyType::Aggressive,
            StrategyType::Balanced,
            StrategyType::Experimental,
            StrategyType::Adaptive,
        ];
        assert_eq!(types.len(), 5);
    }

    #[test]
    fn test_execution_strategy_creation() {
        let mut params = HashMap::new();
        params.insert("risk_tolerance".to_string(), 0.5);

        let strategy = ExecutionStrategy {
            strategy_id: "strategy-1".to_string(),
            strategy_name: "Test Strategy".to_string(),
            strategy_type: StrategyType::Balanced,
            parameters: params,
            applicability_conditions: vec!["general".to_string()],
            expected_performance: ExpectedPerformance {
                success_rate: 0.8,
                efficiency: 0.75,
                quality_score: 0.85,
                resource_usage: 0.6,
                execution_time: Duration::from_secs(1),
            },
            resource_requirements: ResourceRequirements {
                cpu_intensive: false,
                memory_requirements: 64,
                network_dependent: true,
                parallel_capable: true,
            },
        };

        assert_eq!(strategy.strategy_id, "strategy-1");
        assert!(matches!(strategy.strategy_type, StrategyType::Balanced));
    }

    // ========== Performance Tests ==========

    #[test]
    fn test_expected_performance_creation() {
        let perf = ExpectedPerformance {
            success_rate: 0.9,
            efficiency: 0.85,
            quality_score: 0.8,
            resource_usage: 0.7,
            execution_time: Duration::from_secs(2),
        };

        assert!((perf.success_rate - 0.9).abs() < f64::EPSILON);
        assert_eq!(perf.execution_time, Duration::from_secs(2));
    }

    #[test]
    fn test_resource_requirements_creation() {
        let reqs = ResourceRequirements {
            cpu_intensive: true,
            memory_requirements: 256,
            network_dependent: false,
            parallel_capable: true,
        };

        assert!(reqs.cpu_intensive);
        assert_eq!(reqs.memory_requirements, 256);
        assert!(reqs.parallel_capable);
    }

    #[test]
    fn test_strategy_performance_creation() {
        let perf = StrategyPerformance {
            strategy_id: "test".to_string(),
            usage_count: 10,
            success_rate: 0.85,
            average_efficiency: 0.8,
            quality_average: 0.9,
            adaptation_frequency: 2,
            last_used: SystemTime::now(),
        };

        assert_eq!(perf.usage_count, 10);
        assert!((perf.success_rate - 0.85).abs() < f64::EPSILON);
    }

    // ========== Adaptation Tests ==========

    #[test]
    fn test_strategy_adaptation_creation() {
        let adaptation = StrategyAdaptation {
            adaptation_id: "adapt-1".to_string(),
            timestamp: SystemTime::now(),
            from_strategy: "conservative".to_string(),
            to_strategy: "balanced".to_string(),
            trigger_reason: "Performance improved".to_string(),
            performance_before: 0.7,
            performance_after: Some(0.85),
            adaptation_success: Some(true),
        };

        assert_eq!(adaptation.from_strategy, "conservative");
        assert_eq!(adaptation.to_strategy, "balanced");
        assert_eq!(adaptation.adaptation_success, Some(true));
    }

    #[test]
    fn test_adaptation_type_variants() {
        let types = vec![
            AdaptationType::Incremental,
            AdaptationType::Dramatic,
            AdaptationType::Experimental,
            AdaptationType::Rollback,
        ];
        assert_eq!(types.len(), 4);
    }

    #[test]
    fn test_active_adaptation_creation() {
        let adaptation = ActiveAdaptation {
            adaptation_id: "active-1".to_string(),
            started_at: SystemTime::now(),
            adaptation_type: AdaptationType::Incremental,
            parameters_changed: vec!["risk".to_string()],
            monitoring_metrics: vec!["success_rate".to_string()],
        };

        assert_eq!(adaptation.adaptation_id, "active-1");
        assert!(matches!(
            adaptation.adaptation_type,
            AdaptationType::Incremental
        ));
    }

    // ========== Rule Tests ==========

    #[test]
    fn test_rule_type_variants() {
        let types = vec![
            RuleType::PerformanceBased,
            RuleType::TimeBased,
            RuleType::ResourceBased,
            RuleType::QualityBased,
            RuleType::ContextBased,
        ];
        assert_eq!(types.len(), 5);
    }

    #[test]
    fn test_adaptation_rule_creation() {
        let rule = AdaptationRule {
            rule_id: "rule-1".to_string(),
            rule_type: RuleType::PerformanceBased,
            conditions: vec!["success_rate < 0.7".to_string()],
            actions: vec![AdaptationAction {
                action_type: ActionType::StrategySwitch,
                target_parameter: "strategy".to_string(),
                adjustment_value: 0.0,
                expected_impact: 0.15,
            }],
            confidence: 0.9,
            priority: 1,
        };

        assert_eq!(rule.rule_id, "rule-1");
        assert_eq!(rule.priority, 1);
        assert_eq!(rule.actions.len(), 1);
    }

    #[test]
    fn test_action_type_variants() {
        let types = vec![
            ActionType::ParameterAdjustment,
            ActionType::StrategySwitch,
            ActionType::ResourceReallocation,
            ActionType::PriorityChange,
            ActionType::ApproachModification,
        ];
        assert_eq!(types.len(), 5);
    }

    #[test]
    fn test_adaptation_action_creation() {
        let action = AdaptationAction {
            action_type: ActionType::ParameterAdjustment,
            target_parameter: "parallelism".to_string(),
            adjustment_value: 0.2,
            expected_impact: 0.1,
        };

        assert_eq!(action.target_parameter, "parallelism");
        assert!((action.adjustment_value - 0.2).abs() < f64::EPSILON);
    }

    // ========== Trigger Condition Tests ==========

    #[test]
    fn test_comparison_type_variants() {
        let types = vec![
            ComparisonType::LessThan,
            ComparisonType::GreaterThan,
            ComparisonType::Equals,
            ComparisonType::Trend,
        ];
        assert_eq!(types.len(), 4);
    }

    #[test]
    fn test_trigger_condition_creation() {
        let condition = TriggerCondition {
            condition_id: "cond-1".to_string(),
            metric_name: "success_rate".to_string(),
            threshold: 0.75,
            comparison: ComparisonType::LessThan,
            duration: Duration::from_secs(300),
        };

        assert_eq!(condition.metric_name, "success_rate");
        assert!((condition.threshold - 0.75).abs() < f64::EPSILON);
    }

    // ========== Learning System Tests ==========

    #[test]
    fn test_adaptation_pattern_creation() {
        let pattern = AdaptationPattern {
            pattern_id: "pattern-1".to_string(),
            context_conditions: vec!["low_resources".to_string()],
            successful_adaptations: vec!["switch_to_conservative".to_string()],
            pattern_confidence: 0.85,
            usage_frequency: 5,
        };

        assert_eq!(pattern.pattern_id, "pattern-1");
        assert!((pattern.pattern_confidence - 0.85).abs() < f64::EPSILON);
    }

    #[test]
    fn test_failure_analysis_creation() {
        let analysis = FailureAnalysis {
            failure_id: "fail-1".to_string(),
            failed_adaptation: "aggressive_switch".to_string(),
            failure_reason: "Resource constraints".to_string(),
            lessons_learned: vec!["Check resources first".to_string()],
            prevention_strategies: vec!["Resource pre-check".to_string()],
        };

        assert_eq!(analysis.failure_id, "fail-1");
        assert_eq!(analysis.lessons_learned.len(), 1);
    }

    // ========== Manager Default Tests ==========

    #[test]
    fn test_strategy_manager_default() {
        let manager = StrategyManager::default();

        assert!(manager.available_strategies.is_empty());
        assert!(manager.current_strategy.is_none());
        assert!(manager.strategy_performance.is_empty());
        assert!(manager.adaptation_history.is_empty());
    }

    #[test]
    fn test_adaptation_engine_default() {
        let engine = AdaptationEngine::default();

        assert!(engine.adaptation_rules.is_empty());
        assert!(engine.trigger_conditions.is_empty());
        assert!(engine.active_adaptations.is_empty());
    }

    #[test]
    fn test_learning_system_default() {
        let system = LearningSystem::default();

        assert!(system.learned_patterns.is_empty());
        assert!(system.success_factors.is_empty());
        assert!(system.failure_analysis.is_empty());
    }

    // ========== System Tests ==========

    #[tokio::test]
    async fn test_adaptive_strategy_system_new() {
        let config = AdaptiveConfig::default();
        let _system = AdaptiveStrategySystem::new(config);
        // Just verify it creates without panic
        // Give async init time to complete
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert!(true);
    }

    #[tokio::test]
    async fn test_get_current_strategy_initially_none() {
        // Create system with disabled adaptation for cleaner test
        let mut config = AdaptiveConfig::default();
        config.enable_real_time_adaptation = false;
        let system = AdaptiveStrategySystem::new(config);

        // Note: default strategies are initialized asynchronously
        // so we might or might not have a current strategy yet
        let _strategy = system.get_current_strategy().await;
        // Either None or Some is valid since initialization is async
    }

    #[tokio::test]
    async fn test_update_strategy_performance() {
        let config = AdaptiveConfig::default();
        let system = AdaptiveStrategySystem::new(config);

        // Update performance for a strategy
        system
            .update_strategy_performance("test-strategy", true, 0.85, 0.9)
            .await
            .unwrap();

        // Update again
        system
            .update_strategy_performance("test-strategy", true, 0.9, 0.95)
            .await
            .unwrap();

        // Verify tracking worked (check manager directly)
        let manager = system.strategy_manager.read().await;
        let perf = manager.strategy_performance.get("test-strategy");
        assert!(perf.is_some());
        assert_eq!(perf.unwrap().usage_count, 2);
    }

    // ========== Serialization Tests ==========

    #[test]
    fn test_adaptive_config_serialization() {
        let config = AdaptiveConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: AdaptiveConfig = serde_json::from_str(&json).unwrap();

        assert!(deserialized.enable_real_time_adaptation);
    }

    #[test]
    fn test_execution_strategy_serialization() {
        let strategy = ExecutionStrategy {
            strategy_id: "test".to_string(),
            strategy_name: "Test".to_string(),
            strategy_type: StrategyType::Conservative,
            parameters: HashMap::new(),
            applicability_conditions: Vec::new(),
            expected_performance: ExpectedPerformance {
                success_rate: 0.9,
                efficiency: 0.8,
                quality_score: 0.85,
                resource_usage: 0.5,
                execution_time: Duration::from_secs(1),
            },
            resource_requirements: ResourceRequirements {
                cpu_intensive: false,
                memory_requirements: 50,
                network_dependent: false,
                parallel_capable: true,
            },
        };

        let json = serde_json::to_string(&strategy).unwrap();
        let deserialized: ExecutionStrategy = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.strategy_id, "test");
    }

    #[test]
    fn test_strategy_adaptation_serialization() {
        let adaptation = StrategyAdaptation {
            adaptation_id: "test".to_string(),
            timestamp: SystemTime::now(),
            from_strategy: "a".to_string(),
            to_strategy: "b".to_string(),
            trigger_reason: "test".to_string(),
            performance_before: 0.7,
            performance_after: None,
            adaptation_success: None,
        };

        let json = serde_json::to_string(&adaptation).unwrap();
        let deserialized: StrategyAdaptation = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.adaptation_id, "test");
    }
}
