//! Advanced Error Recovery System for Graceful Failure Handling
//!
//! This module provides comprehensive error recovery capabilities for
//! autonomous agents, including failure detection, analysis, recovery
//! strategies, and resilience mechanisms.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::context::ExecutionContext;

use fluent_core::traits::Engine;

/// Advanced error recovery system
pub struct ErrorRecoverySystem {
    config: RecoveryConfig,
    reasoning_engine: Arc<dyn Engine>,
    error_analyzer: Arc<RwLock<ErrorAnalyzer>>,
    recovery_strategies: Arc<RwLock<RecoveryStrategyManager>>,
    failure_history: Arc<RwLock<FailureHistory>>,
    resilience_monitor: Arc<RwLock<ResilienceMonitor>>,
}

/// Configuration for error recovery behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryConfig {
    /// Maximum recovery attempts per failure
    pub max_recovery_attempts: u32,
    /// Enable predictive failure detection
    pub enable_predictive_detection: bool,
    /// Enable automatic strategy adaptation
    pub enable_adaptive_strategies: bool,
    /// Recovery timeout in seconds
    pub recovery_timeout_secs: u64,
    /// Minimum confidence for recovery actions
    pub min_recovery_confidence: f64,
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        Self {
            max_recovery_attempts: 3,
            enable_predictive_detection: true,
            enable_adaptive_strategies: true,
            recovery_timeout_secs: 300,
            min_recovery_confidence: 0.6,
        }
    }
}

/// Error analyzer for failure detection and classification
#[derive(Debug, Default)]
pub struct ErrorAnalyzer {
    detected_errors: Vec<ErrorInstance>,
    error_patterns: HashMap<String, ErrorPattern>,
    failure_predictors: Vec<FailurePredictor>,
    analysis_history: VecDeque<AnalysisResult>,
}

/// Instance of a detected error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorInstance {
    pub error_id: String,
    pub error_type: ErrorType,
    pub severity: ErrorSeverity,
    pub description: String,
    pub context: String,
    pub timestamp: SystemTime,
    pub affected_tasks: Vec<String>,
    pub root_cause: Option<String>,
    pub recovery_suggestions: Vec<String>,
}

/// Classification of error types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ErrorType {
    SystemFailure,
    ResourceExhaustion,
    NetworkTimeout,
    ValidationError,
    LogicalError,
    DependencyFailure,
    ConfigurationError,
    PermissionError,
    UnexpectedState,
    ExternalServiceFailure,
}

/// Severity levels for errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorSeverity {
    Critical, // System cannot continue
    High,     // Major functionality affected
    Medium,   // Some features impacted
    Low,      // Minor issues
    Warning,  // Potential problems
}

/// Pattern of recurring errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPattern {
    pub pattern_id: String,
    pub pattern_type: ErrorType,
    pub frequency: u32,
    pub typical_context: String,
    pub common_causes: Vec<String>,
    pub effective_recoveries: Vec<String>,
    pub prevention_strategies: Vec<String>,
}

/// Predictor for potential failures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailurePredictor {
    pub predictor_id: String,
    pub predictor_type: PredictorType,
    pub confidence: f64,
    pub warning_indicators: Vec<String>,
    pub prediction_horizon: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PredictorType {
    ResourceTrend,
    PerformanceDegradation,
    ErrorRateIncrease,
    DependencyInstability,
    PatternMatching,
}

/// Result of error analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub analysis_id: String,
    pub analyzed_at: SystemTime,
    pub errors_detected: u32,
    pub patterns_identified: u32,
    pub predictions_made: u32,
    pub recovery_recommendations: Vec<String>,
}

/// Manager for recovery strategies
#[derive(Debug, Default)]
pub struct RecoveryStrategyManager {
    strategies: HashMap<String, RecoveryStrategy>,
    strategy_effectiveness: HashMap<String, EffectivenessMetrics>,
    adaptive_policies: Vec<AdaptivePolicy>,
    execution_history: VecDeque<StrategyExecution>,
}

/// Recovery strategy definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryStrategy {
    pub strategy_id: String,
    pub strategy_name: String,
    pub applicable_errors: Vec<ErrorType>,
    pub recovery_actions: Vec<RecoveryAction>,
    pub success_criteria: Vec<String>,
    pub rollback_actions: Vec<RecoveryAction>,
    pub estimated_time: Duration,
    pub confidence_score: f64,
}

/// Individual recovery action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryAction {
    pub action_id: String,
    pub action_type: ActionType,
    pub description: String,
    pub parameters: HashMap<String, String>,
    pub timeout: Duration,
    pub retry_policy: RetryPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionType {
    Restart,
    Rollback,
    ResourceCleanup,
    ConfigurationFix,
    DependencyReset,
    StateCorrection,
    AlternativeExecution,
    GracefulDegradation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub retry_delay: Duration,
    pub backoff_multiplier: f64,
}

/// Metrics for strategy effectiveness
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectivenessMetrics {
    pub success_rate: f64,
    pub average_recovery_time: Duration,
    pub resource_efficiency: f64,
    pub side_effect_frequency: f64,
    pub usage_count: u32,
}

/// Adaptive policy for strategy selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptivePolicy {
    pub policy_id: String,
    pub conditions: Vec<String>,
    pub strategy_preferences: Vec<String>,
    pub adaptation_rules: Vec<String>,
}

/// Record of strategy execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyExecution {
    pub execution_id: String,
    pub strategy_id: String,
    pub error_id: String,
    pub started_at: SystemTime,
    pub completed_at: Option<SystemTime>,
    pub success: bool,
    pub actions_taken: Vec<String>,
    pub outcome_description: String,
}

/// History of failures and recoveries
#[derive(Debug, Default)]
pub struct FailureHistory {
    incidents: VecDeque<FailureIncident>,
    recovery_statistics: RecoveryStatistics,
    learning_insights: Vec<LearningInsight>,
}

/// Record of a failure incident
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureIncident {
    pub incident_id: String,
    pub error_instance: ErrorInstance,
    pub recovery_attempts: Vec<RecoveryAttempt>,
    pub final_outcome: IncidentOutcome,
    pub lessons_learned: Vec<String>,
    pub prevention_recommendations: Vec<String>,
}

/// Individual recovery attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryAttempt {
    pub attempt_id: String,
    pub strategy_used: String,
    pub started_at: SystemTime,
    pub duration: Duration,
    pub success: bool,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IncidentOutcome {
    FullRecovery,
    PartialRecovery,
    GracefulDegradation,
    CompleteFailure,
    ManualIntervention,
}

/// Statistics about recovery performance
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RecoveryStatistics {
    pub total_incidents: u32,
    pub successful_recoveries: u32,
    pub average_recovery_time: Duration,
    pub most_common_errors: Vec<ErrorType>,
    pub most_effective_strategies: Vec<String>,
}

/// Insight learned from failures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningInsight {
    pub insight_id: String,
    pub insight_type: InsightType,
    pub description: String,
    pub applicability: Vec<String>,
    pub confidence: f64,
    pub derived_from: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InsightType {
    PreventionStrategy,
    ImprovedDetection,
    BetterRecovery,
    SystemWeakness,
    OperationalGuidance,
}

/// Monitor for system resilience
#[derive(Debug, Default)]
pub struct ResilienceMonitor {
    resilience_metrics: ResilienceMetrics,
    health_indicators: Vec<HealthIndicator>,
    stress_tests: Vec<StressTestResult>,
    improvement_suggestions: Vec<ImprovementSuggestion>,
}

/// Metrics measuring system resilience
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ResilienceMetrics {
    pub mean_time_to_failure: Duration,
    pub mean_time_to_recovery: Duration,
    pub availability_percentage: f64,
    pub fault_tolerance_score: f64,
    pub adaptability_index: f64,
}

/// Health indicator for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthIndicator {
    pub indicator_id: String,
    pub indicator_name: String,
    pub current_value: f64,
    pub threshold_warning: f64,
    pub threshold_critical: f64,
    pub trend: HealthTrend,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthTrend {
    Improving,
    Stable,
    Degrading,
    Critical,
}

/// Result of resilience stress testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressTestResult {
    pub test_id: String,
    pub test_scenario: String,
    pub performed_at: SystemTime,
    pub failures_induced: u32,
    pub recovery_success_rate: f64,
    pub performance_impact: f64,
    pub identified_weaknesses: Vec<String>,
}

/// Suggestion for improving resilience
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovementSuggestion {
    pub suggestion_id: String,
    pub improvement_type: ImprovementType,
    pub description: String,
    pub expected_benefit: f64,
    pub implementation_effort: f64,
    pub priority: Priority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImprovementType {
    RedundancyAddition,
    MonitoringEnhancement,
    RecoveryOptimization,
    PreventionMeasure,
    PerformanceImprovement,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Priority {
    Critical,
    High,
    Medium,
    Low,
}

/// Result of error recovery operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryResult {
    pub recovery_id: String,
    pub error_id: String,
    pub strategy_used: String,
    pub success: bool,
    pub recovery_time: Duration,
    pub actions_performed: Vec<String>,
    pub final_state: String,
    pub lessons_learned: Vec<String>,
}

impl ErrorRecoverySystem {
    /// Create a new error recovery system
    pub fn new(engine: Arc<dyn Engine>, config: RecoveryConfig) -> Self {
        Self {
            config,
            reasoning_engine: engine,
            error_analyzer: Arc::new(RwLock::new(ErrorAnalyzer::default())),
            recovery_strategies: Arc::new(RwLock::new(RecoveryStrategyManager::default())),
            failure_history: Arc::new(RwLock::new(FailureHistory::default())),
            resilience_monitor: Arc::new(RwLock::new(ResilienceMonitor::default())),
        }
    }

    /// Initialize with default recovery strategies
    pub async fn initialize_strategies(&self) -> Result<()> {
        let mut manager = self.recovery_strategies.write().await;

        // Add default strategies
        manager.strategies.insert(
            "restart".to_string(),
            RecoveryStrategy {
                strategy_id: "restart".to_string(),
                strategy_name: "System Restart".to_string(),
                applicable_errors: vec![ErrorType::SystemFailure, ErrorType::UnexpectedState],
                recovery_actions: vec![RecoveryAction {
                    action_id: Uuid::new_v4().to_string(),
                    action_type: ActionType::Restart,
                    description: "Restart failed component".to_string(),
                    parameters: HashMap::new(),
                    timeout: Duration::from_secs(60),
                    retry_policy: RetryPolicy {
                        max_retries: 2,
                        retry_delay: Duration::from_secs(10),
                        backoff_multiplier: 2.0,
                    },
                }],
                success_criteria: vec!["Component responds normally".to_string()],
                rollback_actions: Vec::new(),
                estimated_time: Duration::from_secs(120),
                confidence_score: 0.8,
            },
        );

        manager.strategies.insert(
            "rollback".to_string(),
            RecoveryStrategy {
                strategy_id: "rollback".to_string(),
                strategy_name: "State Rollback".to_string(),
                applicable_errors: vec![ErrorType::ValidationError, ErrorType::LogicalError],
                recovery_actions: vec![RecoveryAction {
                    action_id: Uuid::new_v4().to_string(),
                    action_type: ActionType::Rollback,
                    description: "Rollback to previous stable state".to_string(),
                    parameters: HashMap::new(),
                    timeout: Duration::from_secs(30),
                    retry_policy: RetryPolicy {
                        max_retries: 1,
                        retry_delay: Duration::from_secs(5),
                        backoff_multiplier: 1.0,
                    },
                }],
                success_criteria: vec!["System in stable state".to_string()],
                rollback_actions: Vec::new(),
                estimated_time: Duration::from_secs(60),
                confidence_score: 0.9,
            },
        );

        Ok(())
    }

    /// Handle an error with automatic recovery
    pub async fn handle_error(
        &self,
        error: ErrorInstance,
        context: &ExecutionContext,
    ) -> Result<RecoveryResult> {
        let recovery_id = Uuid::new_v4().to_string();

        // Analyze the error
        self.analyze_error(&error).await?;

        // Select recovery strategy
        let strategy = self.select_recovery_strategy(&error).await?;

        // Execute recovery
        let result = self
            .execute_recovery(&recovery_id, &error, &strategy, context)
            .await?;

        // Record the incident
        self.record_incident(&error, &result).await?;

        // Update resilience metrics
        self.update_resilience_metrics(&result).await?;

        Ok(result)
    }

    /// Analyze an error to understand its nature and impact
    async fn analyze_error(&self, error: &ErrorInstance) -> Result<()> {
        let mut analyzer = self.error_analyzer.write().await;

        // Store the error instance
        analyzer.detected_errors.push(error.clone());

        // Look for patterns
        let pattern_key = format!("{:?}", error.error_type);
        analyzer
            .error_patterns
            .entry(pattern_key.clone())
            .and_modify(|pattern| pattern.frequency += 1)
            .or_insert(ErrorPattern {
                pattern_id: Uuid::new_v4().to_string(),
                pattern_type: error.error_type.clone(),
                frequency: 1,
                typical_context: error.context.clone(),
                common_causes: vec![error
                    .root_cause
                    .clone()
                    .unwrap_or_else(|| "Unknown".to_string())],
                effective_recoveries: Vec::new(),
                prevention_strategies: Vec::new(),
            });

        let patterns_count = analyzer.error_patterns.len();

        // Record analysis
        analyzer.analysis_history.push_back(AnalysisResult {
            analysis_id: Uuid::new_v4().to_string(),
            analyzed_at: SystemTime::now(),
            errors_detected: 1,
            patterns_identified: patterns_count as u32,
            predictions_made: 0,
            recovery_recommendations: error.recovery_suggestions.clone(),
        });

        // Keep history manageable
        if analyzer.analysis_history.len() > 1000 {
            analyzer.analysis_history.pop_front();
        }

        Ok(())
    }

    /// Select the best recovery strategy for an error
    async fn select_recovery_strategy(&self, error: &ErrorInstance) -> Result<RecoveryStrategy> {
        let manager = self.recovery_strategies.read().await;

        // Find applicable strategies
        let mut candidates: Vec<&RecoveryStrategy> = manager
            .strategies
            .values()
            .filter(|s| s.applicable_errors.contains(&error.error_type))
            .collect();

        if candidates.is_empty() {
            return self.create_default_strategy(error).await;
        }

        // Sort by confidence score and effectiveness
        candidates.sort_by(|a, b| {
            let eff_a = manager
                .strategy_effectiveness
                .get(&a.strategy_id)
                .map(|e| e.success_rate)
                .unwrap_or(0.5);
            let eff_b = manager
                .strategy_effectiveness
                .get(&b.strategy_id)
                .map(|e| e.success_rate)
                .unwrap_or(0.5);

            (b.confidence_score * eff_b)
                .partial_cmp(&(a.confidence_score * eff_a))
                .unwrap()
        });

        Ok(candidates[0].clone())
    }

    /// Create a default recovery strategy when none match
    async fn create_default_strategy(&self, error: &ErrorInstance) -> Result<RecoveryStrategy> {
        Ok(RecoveryStrategy {
            strategy_id: "default".to_string(),
            strategy_name: "Default Recovery".to_string(),
            applicable_errors: vec![error.error_type.clone()],
            recovery_actions: vec![RecoveryAction {
                action_id: Uuid::new_v4().to_string(),
                action_type: ActionType::GracefulDegradation,
                description: "Attempt graceful degradation".to_string(),
                parameters: HashMap::new(),
                timeout: Duration::from_secs(60),
                retry_policy: RetryPolicy {
                    max_retries: 1,
                    retry_delay: Duration::from_secs(5),
                    backoff_multiplier: 1.0,
                },
            }],
            success_criteria: vec!["System continues operating".to_string()],
            rollback_actions: Vec::new(),
            estimated_time: Duration::from_secs(90),
            confidence_score: 0.4,
        })
    }

    /// Execute a recovery strategy
    async fn execute_recovery(
        &self,
        recovery_id: &str,
        error: &ErrorInstance,
        strategy: &RecoveryStrategy,
        _context: &ExecutionContext,
    ) -> Result<RecoveryResult> {
        let start_time = SystemTime::now();
        let mut actions_performed = Vec::new();
        let mut success = true;

        // Execute each action in the strategy
        for action in &strategy.recovery_actions {
            match self.execute_action(action).await {
                Ok(description) => {
                    actions_performed.push(description);
                }
                Err(_) => {
                    success = false;
                    break;
                }
            }
        }

        let recovery_time = SystemTime::now()
            .duration_since(start_time)
            .unwrap_or_default();

        Ok(RecoveryResult {
            recovery_id: recovery_id.to_string(),
            error_id: error.error_id.clone(),
            strategy_used: strategy.strategy_id.clone(),
            success,
            recovery_time,
            actions_performed,
            final_state: if success { "Recovered" } else { "Failed" }.to_string(),
            lessons_learned: vec!["Recovery strategy executed".to_string()],
        })
    }

    /// Execute a single recovery action
    async fn execute_action(&self, action: &RecoveryAction) -> Result<String> {
        match action.action_type {
            ActionType::Restart => {
                // Simulate restart action
                tokio::time::sleep(Duration::from_secs(2)).await;
                Ok("Component restarted successfully".to_string())
            }
            ActionType::Rollback => {
                // Simulate rollback action
                tokio::time::sleep(Duration::from_secs(1)).await;
                Ok("State rolled back successfully".to_string())
            }
            ActionType::GracefulDegradation => Ok("Graceful degradation applied".to_string()),
            _ => Ok(format!("Executed {:?} action", action.action_type)),
        }
    }

    /// Record a failure incident
    async fn record_incident(&self, error: &ErrorInstance, result: &RecoveryResult) -> Result<()> {
        let mut history = self.failure_history.write().await;

        let incident = FailureIncident {
            incident_id: Uuid::new_v4().to_string(),
            error_instance: error.clone(),
            recovery_attempts: vec![RecoveryAttempt {
                attempt_id: result.recovery_id.clone(),
                strategy_used: result.strategy_used.clone(),
                started_at: SystemTime::now() - result.recovery_time,
                duration: result.recovery_time,
                success: result.success,
                notes: result.final_state.clone(),
            }],
            final_outcome: if result.success {
                IncidentOutcome::FullRecovery
            } else {
                IncidentOutcome::CompleteFailure
            },
            lessons_learned: result.lessons_learned.clone(),
            prevention_recommendations: Vec::new(),
        };

        history.incidents.push_back(incident);

        // Update statistics
        history.recovery_statistics.total_incidents += 1;
        if result.success {
            history.recovery_statistics.successful_recoveries += 1;
        }

        // Keep history manageable
        if history.incidents.len() > 1000 {
            history.incidents.pop_front();
        }

        Ok(())
    }

    /// Update resilience metrics based on recovery results
    async fn update_resilience_metrics(&self, result: &RecoveryResult) -> Result<()> {
        let mut monitor = self.resilience_monitor.write().await;

        // Update recovery time metrics
        let current_mttr = monitor.resilience_metrics.mean_time_to_recovery;
        monitor.resilience_metrics.mean_time_to_recovery = if current_mttr == Duration::from_secs(0)
        {
            result.recovery_time
        } else {
            Duration::from_secs((current_mttr.as_secs() + result.recovery_time.as_secs()) / 2)
        };

        // Update availability if recovery was successful
        if result.success {
            monitor.resilience_metrics.availability_percentage =
                (monitor.resilience_metrics.availability_percentage + 0.99) / 2.0;
        }

        Ok(())
    }

    /// Get current resilience metrics
    pub async fn get_resilience_metrics(&self) -> Result<ResilienceMetrics> {
        let monitor = self.resilience_monitor.read().await;
        Ok(monitor.resilience_metrics.clone())
    }

    /// Get error analysis summary
    pub async fn get_error_summary(&self) -> Result<AnalysisResult> {
        let analyzer = self.error_analyzer.read().await;
        if let Some(latest) = analyzer.analysis_history.back() {
            Ok(latest.clone())
        } else {
            Ok(AnalysisResult {
                analysis_id: "empty".to_string(),
                analyzed_at: SystemTime::now(),
                errors_detected: 0,
                patterns_identified: 0,
                predictions_made: 0,
                recovery_recommendations: Vec::new(),
            })
        }
    }

    /// Predict potential failures based on current system state
    pub async fn predict_failures(&self) -> Result<Vec<FailurePredictor>> {
        if !self.config.enable_predictive_detection {
            return Ok(Vec::new());
        }

        let mut analyzer = self.error_analyzer.write().await;
        let mut predictions = Vec::new();

        // Analyze error patterns for increasing frequency
        for (pattern_key, pattern) in &analyzer.error_patterns {
            if pattern.frequency >= 3 {
                predictions.push(FailurePredictor {
                    predictor_id: Uuid::new_v4().to_string(),
                    predictor_type: PredictorType::ErrorRateIncrease,
                    confidence: (pattern.frequency as f64 / 10.0).min(0.9),
                    warning_indicators: vec![
                        format!(
                            "Error type {:?} occurred {} times",
                            pattern.pattern_type, pattern.frequency
                        ),
                        format!("Common context: {}", pattern.typical_context),
                    ],
                    prediction_horizon: Duration::from_secs(300),
                });
            }
        }

        // Check for pattern-based predictions
        let history = self.failure_history.read().await;
        let recent_incidents: Vec<_> = history
            .incidents
            .iter()
            .filter(|i| {
                i.error_instance
                    .timestamp
                    .elapsed()
                    .map(|d| d < Duration::from_secs(3600))
                    .unwrap_or(false)
            })
            .collect();

        if recent_incidents.len() >= 5 {
            predictions.push(FailurePredictor {
                predictor_id: Uuid::new_v4().to_string(),
                predictor_type: PredictorType::PatternMatching,
                confidence: 0.7,
                warning_indicators: vec![format!(
                    "{} incidents in the last hour",
                    recent_incidents.len()
                )],
                prediction_horizon: Duration::from_secs(600),
            });
        }

        // Store predictors for future reference
        analyzer.failure_predictors = predictions.clone();

        Ok(predictions)
    }

    /// Update health indicators based on current system state
    pub async fn update_health_indicators(&self) -> Result<Vec<HealthIndicator>> {
        let mut monitor = self.resilience_monitor.write().await;
        let history = self.failure_history.read().await;

        let mut indicators = Vec::new();

        // Error rate indicator
        let recent_errors = history
            .incidents
            .iter()
            .filter(|i| {
                i.error_instance
                    .timestamp
                    .elapsed()
                    .map(|d| d < Duration::from_secs(3600))
                    .unwrap_or(false)
            })
            .count();

        let error_rate = recent_errors as f64;
        indicators.push(HealthIndicator {
            indicator_id: "error_rate".to_string(),
            indicator_name: "Error Rate (per hour)".to_string(),
            current_value: error_rate,
            threshold_warning: 5.0,
            threshold_critical: 10.0,
            trend: if error_rate > 10.0 {
                HealthTrend::Critical
            } else if error_rate > 5.0 {
                HealthTrend::Degrading
            } else if error_rate > 2.0 {
                HealthTrend::Stable
            } else {
                HealthTrend::Improving
            },
        });

        // Recovery success rate indicator
        let total = history.recovery_statistics.total_incidents;
        let success_rate = if total > 0 {
            history.recovery_statistics.successful_recoveries as f64 / total as f64
        } else {
            1.0
        };

        indicators.push(HealthIndicator {
            indicator_id: "recovery_success".to_string(),
            indicator_name: "Recovery Success Rate".to_string(),
            current_value: success_rate,
            threshold_warning: 0.7,
            threshold_critical: 0.5,
            trend: if success_rate < 0.5 {
                HealthTrend::Critical
            } else if success_rate < 0.7 {
                HealthTrend::Degrading
            } else if success_rate < 0.9 {
                HealthTrend::Stable
            } else {
                HealthTrend::Improving
            },
        });

        // Mean time to recovery indicator
        let mttr_secs = monitor.resilience_metrics.mean_time_to_recovery.as_secs() as f64;
        indicators.push(HealthIndicator {
            indicator_id: "mttr".to_string(),
            indicator_name: "Mean Time to Recovery (seconds)".to_string(),
            current_value: mttr_secs,
            threshold_warning: 60.0,
            threshold_critical: 180.0,
            trend: if mttr_secs > 180.0 {
                HealthTrend::Critical
            } else if mttr_secs > 60.0 {
                HealthTrend::Degrading
            } else if mttr_secs > 30.0 {
                HealthTrend::Stable
            } else {
                HealthTrend::Improving
            },
        });

        monitor.health_indicators = indicators.clone();
        Ok(indicators)
    }

    /// Generate learning insights from failure history
    pub async fn generate_learning_insights(&self) -> Result<Vec<LearningInsight>> {
        let mut history = self.failure_history.write().await;
        let analyzer = self.error_analyzer.read().await;
        let mut insights = Vec::new();

        // Insight: Most common error types
        if !analyzer.error_patterns.is_empty() {
            let mut patterns: Vec<_> = analyzer.error_patterns.values().collect();
            patterns.sort_by(|a, b| b.frequency.cmp(&a.frequency));

            if let Some(most_common) = patterns.first() {
                if most_common.frequency >= 3 {
                    insights.push(LearningInsight {
                        insight_id: Uuid::new_v4().to_string(),
                        insight_type: InsightType::SystemWeakness,
                        description: format!(
                            "Error type {:?} is most frequent ({} occurrences). Consider implementing preventive measures.",
                            most_common.pattern_type, most_common.frequency
                        ),
                        applicability: vec![format!("{:?}", most_common.pattern_type)],
                        confidence: 0.8,
                        derived_from: vec!["error_pattern_analysis".to_string()],
                    });
                }
            }
        }

        // Insight: Effective recovery strategies
        let strategies = self.recovery_strategies.read().await;
        for (strategy_id, effectiveness) in &strategies.strategy_effectiveness {
            if effectiveness.success_rate > 0.8 && effectiveness.usage_count >= 3 {
                insights.push(LearningInsight {
                    insight_id: Uuid::new_v4().to_string(),
                    insight_type: InsightType::BetterRecovery,
                    description: format!(
                        "Strategy '{}' is highly effective ({:.0}% success rate over {} uses)",
                        strategy_id,
                        effectiveness.success_rate * 100.0,
                        effectiveness.usage_count
                    ),
                    applicability: vec![strategy_id.clone()],
                    confidence: effectiveness.success_rate,
                    derived_from: vec!["strategy_effectiveness_analysis".to_string()],
                });
            }
        }

        // Insight: Prevention recommendations from repeated failures
        let mut error_contexts: HashMap<String, u32> = HashMap::new();
        for incident in history.incidents.iter() {
            *error_contexts
                .entry(incident.error_instance.context.clone())
                .or_insert(0) += 1;
        }

        for (context, count) in error_contexts {
            if count >= 2 {
                insights.push(LearningInsight {
                    insight_id: Uuid::new_v4().to_string(),
                    insight_type: InsightType::PreventionStrategy,
                    description: format!(
                        "Context '{}' has caused {} failures. Consider adding validation or guards.",
                        context, count
                    ),
                    applicability: vec![context],
                    confidence: 0.7,
                    derived_from: vec!["incident_context_analysis".to_string()],
                });
            }
        }

        history.learning_insights = insights.clone();
        Ok(insights)
    }

    /// Apply adaptive policy to strategy selection
    pub async fn apply_adaptive_policy(&self, error: &ErrorInstance) -> Result<Option<String>> {
        if !self.config.enable_adaptive_strategies {
            return Ok(None);
        }

        let manager = self.recovery_strategies.read().await;

        // Find applicable adaptive policies
        for policy in &manager.adaptive_policies {
            // Check if conditions match
            let conditions_met = policy.conditions.iter().all(|condition| {
                // Simple condition matching based on error context
                error.context.contains(condition)
                    || format!("{:?}", error.error_type).contains(condition)
                    || error.description.contains(condition)
            });

            if conditions_met && !policy.strategy_preferences.is_empty() {
                // Return the first preferred strategy
                return Ok(Some(policy.strategy_preferences[0].clone()));
            }
        }

        Ok(None)
    }

    /// Register an adaptive policy for strategy selection
    pub async fn register_adaptive_policy(&self, policy: AdaptivePolicy) -> Result<()> {
        let mut manager = self.recovery_strategies.write().await;
        manager.adaptive_policies.push(policy);
        Ok(())
    }

    /// Update strategy effectiveness metrics after a recovery attempt
    pub async fn update_strategy_effectiveness(
        &self,
        strategy_id: &str,
        success: bool,
        recovery_time: Duration,
    ) -> Result<()> {
        let mut manager = self.recovery_strategies.write().await;

        let metrics = manager
            .strategy_effectiveness
            .entry(strategy_id.to_string())
            .or_insert(EffectivenessMetrics {
                success_rate: 0.5,
                average_recovery_time: Duration::from_secs(0),
                resource_efficiency: 0.5,
                side_effect_frequency: 0.0,
                usage_count: 0,
            });

        // Update metrics with exponential moving average
        let alpha = 0.3;
        metrics.success_rate =
            metrics.success_rate * (1.0 - alpha) + (if success { 1.0 } else { 0.0 }) * alpha;

        let current_avg = metrics.average_recovery_time.as_secs_f64();
        let new_avg = current_avg * (1.0 - alpha) + recovery_time.as_secs_f64() * alpha;
        metrics.average_recovery_time = Duration::from_secs_f64(new_avg);

        metrics.usage_count += 1;

        Ok(())
    }

    /// Get improvement suggestions based on current system state
    pub async fn get_improvement_suggestions(&self) -> Result<Vec<ImprovementSuggestion>> {
        let monitor = self.resilience_monitor.read().await;
        let history = self.failure_history.read().await;
        let mut suggestions = Vec::new();

        // Suggestion based on low success rate
        let total = history.recovery_statistics.total_incidents;
        let success_rate = if total > 0 {
            history.recovery_statistics.successful_recoveries as f64 / total as f64
        } else {
            1.0
        };

        if success_rate < 0.7 && total >= 5 {
            suggestions.push(ImprovementSuggestion {
                suggestion_id: Uuid::new_v4().to_string(),
                improvement_type: ImprovementType::RecoveryOptimization,
                description: format!(
                    "Recovery success rate is {:.0}%. Consider adding more recovery strategies or improving existing ones.",
                    success_rate * 100.0
                ),
                expected_benefit: 0.3,
                implementation_effort: 0.5,
                priority: Priority::High,
            });
        }

        // Suggestion based on high MTTR
        let mttr = monitor.resilience_metrics.mean_time_to_recovery;
        if mttr > Duration::from_secs(60) {
            suggestions.push(ImprovementSuggestion {
                suggestion_id: Uuid::new_v4().to_string(),
                improvement_type: ImprovementType::PerformanceImprovement,
                description: format!(
                    "Mean time to recovery is {} seconds. Consider optimizing recovery actions or adding faster alternatives.",
                    mttr.as_secs()
                ),
                expected_benefit: 0.4,
                implementation_effort: 0.6,
                priority: Priority::Medium,
            });
        }

        // Suggestion based on availability
        let availability = monitor.resilience_metrics.availability_percentage;
        if availability < 0.95 {
            suggestions.push(ImprovementSuggestion {
                suggestion_id: Uuid::new_v4().to_string(),
                improvement_type: ImprovementType::RedundancyAddition,
                description: format!(
                    "System availability is {:.1}%. Consider adding redundancy or failover mechanisms.",
                    availability * 100.0
                ),
                expected_benefit: 0.5,
                implementation_effort: 0.7,
                priority: Priority::High,
            });
        }

        // Suggestion for monitoring enhancement
        if monitor.health_indicators.is_empty() {
            suggestions.push(ImprovementSuggestion {
                suggestion_id: Uuid::new_v4().to_string(),
                improvement_type: ImprovementType::MonitoringEnhancement,
                description: "No health indicators configured. Call update_health_indicators() regularly for better monitoring.".to_string(),
                expected_benefit: 0.3,
                implementation_effort: 0.2,
                priority: Priority::Medium,
            });
        }

        Ok(suggestions)
    }

    /// Get recovery statistics
    pub async fn get_recovery_statistics(&self) -> Result<RecoveryStatistics> {
        let history = self.failure_history.read().await;
        Ok(history.recovery_statistics.clone())
    }

    /// Get all health indicators
    pub async fn get_health_indicators(&self) -> Result<Vec<HealthIndicator>> {
        let monitor = self.resilience_monitor.read().await;
        Ok(monitor.health_indicators.clone())
    }

    /// Clear old incidents from history (keep last N)
    pub async fn prune_history(&self, keep_last: usize) -> Result<u32> {
        let mut history = self.failure_history.write().await;
        let original_len = history.incidents.len();

        while history.incidents.len() > keep_last {
            history.incidents.pop_front();
        }

        Ok((original_len - history.incidents.len()) as u32)
    }
}
