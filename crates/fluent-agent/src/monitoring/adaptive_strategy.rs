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
            if let Err(e) = AdaptiveStrategySystem::populate_default_strategies(&mut *manager).await {
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
        if performance.execution_metrics.success_rate < 0.7 || 
           performance.efficiency_metrics.overall_efficiency < 0.6 {
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
        let current_strategy_id = manager.current_strategy
            .as_ref()
            .map(|s| s.strategy_id.clone())
            .unwrap_or_else(|| "default".to_string());
        
        // Find strategy with best expected performance
        let best_strategy = manager.available_strategies.iter()
            .filter(|s| s.strategy_id != current_strategy_id)
            .max_by(|a, b| {
                a.expected_performance.success_rate
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
            trigger_reason: format!("Performance below threshold: {:.2}", 
                performance.execution_metrics.success_rate),
            performance_before: performance.execution_metrics.success_rate,
            performance_after: None,
            adaptation_success: None,
        })
    }

    /// Execute the planned adaptation
    async fn execute_adaptation(&self, adaptation: &StrategyAdaptation) -> Result<()> {
        let mut manager = self.strategy_manager.write().await;
        
        // Find and switch to new strategy
        if let Some(new_strategy) = manager.available_strategies.iter()
            .find(|s| s.strategy_id == adaptation.to_strategy).cloned() {
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
        
        let performance = manager.strategy_performance
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
        
        performance.success_rate = performance.success_rate * (1.0 - alpha) + 
            (if success { 1.0 } else { 0.0 }) * alpha;
        performance.average_efficiency = performance.average_efficiency * (1.0 - alpha) + 
            efficiency * alpha;
        performance.quality_average = performance.quality_average * (1.0 - alpha) + 
            quality * alpha;
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
        if let Some(balanced_strategy) = manager.available_strategies.iter()
            .find(|s| s.strategy_id == "balanced").cloned() {
            manager.current_strategy = Some(balanced_strategy);
        }
        
        Ok(())
    }
    
    async fn initialize_default_strategies(&self) -> Result<()> {
        let mut manager = self.strategy_manager.write().await;
        Self::populate_default_strategies(&mut *manager).await
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