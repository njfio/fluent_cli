//! Dynamic Replanner for Adaptive Planning
//!
//! This module provides sophisticated dynamic replanning capabilities that
//! adapt execution plans based on intermediate results, changing conditions,
//! and performance feedback during autonomous execution.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::task::TaskResult;
use crate::planning::{ExecutionPlan, ScheduledTask};

/// Dynamic replanner for adaptive execution planning
pub struct DynamicReplanner {
    config: ReplannerConfig,
    current_plan: Arc<RwLock<ActivePlan>>,
    execution_monitor: Arc<RwLock<ExecutionMonitor>>,
    adaptation_engine: Arc<RwLock<AdaptationEngine>>,
}

/// Configuration for dynamic replanning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplannerConfig {
    /// Enable automatic replanning when conditions change
    pub enable_auto_replan: bool,
    /// Threshold for triggering replanning (0.0-1.0)
    pub replan_threshold: f64,
    /// Maximum number of replanning attempts
    pub max_replan_attempts: u32,
    /// Minimum time between replanning attempts (seconds)
    pub min_replan_interval: u64,
    /// Enable predictive replanning based on trends
    pub enable_predictive_replan: bool,
    /// Enable resource-aware replanning
    pub enable_resource_replan: bool,
}

impl Default for ReplannerConfig {
    fn default() -> Self {
        Self {
            enable_auto_replan: true,
            replan_threshold: 0.3,
            max_replan_attempts: 5,
            min_replan_interval: 60,
            enable_predictive_replan: true,
            enable_resource_replan: true,
        }
    }
}

/// Currently active execution plan with dynamic state
#[derive(Debug, Default)]
pub struct ActivePlan {
    plan_id: String,
    original_plan: Option<ExecutionPlan>,
    current_schedule: Vec<ScheduledTask>,
    completed_tasks: Vec<String>,
    failed_tasks: Vec<String>,
    in_progress_tasks: Vec<String>,
    pending_tasks: Vec<String>,
    plan_modifications: Vec<PlanModification>,
    current_phase: u32,
    plan_start_time: Option<SystemTime>,
    last_replan_time: Option<SystemTime>,
    plan_performance: PlanPerformance,
}

/// Modification made to the original plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanModification {
    pub modification_id: String,
    pub modification_type: ModificationType,
    pub timestamp: SystemTime,
    pub reason: String,
    pub affected_tasks: Vec<String>,
    pub performance_impact: f64,
}

/// Types of plan modifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModificationType {
    TaskReorder,
    TaskAddition,
    TaskRemoval,
    TaskSplit,
    TaskMerge,
    ParallelismAdjustment,
    ResourceReallocation,
    TimelineAdjustment,
    EmergencyReschedule,
}

/// Performance tracking for the active plan
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PlanPerformance {
    pub completion_rate: f64,
    pub average_task_duration: Duration,
    pub schedule_adherence: f64,
    pub resource_utilization: f64,
    pub bottleneck_frequency: u32,
    pub adaptation_effectiveness: f64,
}

/// Monitor for tracking execution progress and conditions
#[derive(Debug, Default)]
pub struct ExecutionMonitor {
    task_progress: HashMap<String, TaskProgress>,
    resource_usage: HashMap<String, ResourceUsage>,
    performance_metrics: PerformanceMetrics,
    condition_triggers: Vec<ReplanTrigger>,
    monitoring_start: Option<SystemTime>,
}

/// Progress tracking for individual tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProgress {
    pub task_id: String,
    pub status: TaskExecutionStatus,
    pub progress_percentage: f64,
    pub actual_start_time: Option<SystemTime>,
    pub estimated_completion: Option<SystemTime>,
    pub actual_completion: Option<SystemTime>,
    pub performance_issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskExecutionStatus {
    Waiting,
    Starting,
    InProgress,
    Completing,
    Completed,
    Failed,
    Blocked,
    Cancelled,
}

/// Resource usage tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub resource_id: String,
    pub current_utilization: f64,
    pub peak_utilization: f64,
    pub availability_forecast: Vec<AvailabilityPeriod>,
    pub contention_events: u32,
}

/// Period of resource availability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailabilityPeriod {
    pub start_time: Duration,
    pub end_time: Duration,
    pub available_capacity: f64,
}

/// Performance metrics for monitoring
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub throughput: f64,
    pub latency: Duration,
    pub error_rate: f64,
    pub efficiency: f64,
    pub trend_direction: TrendDirection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrendDirection {
    Improving,
    Stable,
    Degrading,
    Volatile,
}

impl Default for TrendDirection {
    fn default() -> Self {
        Self::Stable
    }
}

/// Trigger conditions for replanning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplanTrigger {
    pub trigger_id: String,
    pub trigger_type: TriggerType,
    pub condition: String,
    pub threshold: f64,
    pub current_value: f64,
    pub triggered_at: Option<SystemTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TriggerType {
    PerformanceDegradation,
    ResourceContention,
    TaskFailure,
    ScheduleDeviation,
    ExternalChange,
    UserRequest,
    PredictiveAlert,
}

/// Engine for determining when and how to adapt plans
#[derive(Debug, Default)]
pub struct AdaptationEngine {
    adaptation_strategies: Vec<AdaptationStrategy>,
    strategy_effectiveness: HashMap<String, f64>,
    recent_adaptations: VecDeque<AdaptationEvent>,
}

/// Strategy for adapting plans
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptationStrategy {
    pub strategy_id: String,
    pub strategy_name: String,
    pub applicable_triggers: Vec<TriggerType>,
    pub adaptation_actions: Vec<AdaptationAction>,
    pub success_rate: f64,
    pub implementation_complexity: f64,
}

/// Specific action to adapt a plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptationAction {
    pub action_type: ActionType,
    pub parameters: HashMap<String, String>,
    pub expected_impact: f64,
    pub risk_level: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionType {
    ReorderTasks,
    AdjustParallelism,
    ReallocateResources,
    ModifySchedule,
    AddAlternativeTask,
    RemoveBlockingTask,
    SplitComplexTask,
    MergeSimilarTasks,
}

/// Record of an adaptation event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptationEvent {
    pub event_id: String,
    pub timestamp: SystemTime,
    pub trigger_reason: String,
    pub adaptation_applied: String,
    pub performance_before: f64,
    pub performance_after: Option<f64>,
    pub success: Option<bool>,
}

/// Result of dynamic replanning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplanningResult {
    pub new_plan: ExecutionPlan,
    pub modifications: Vec<PlanModification>,
    pub rationale: String,
    pub expected_improvement: f64,
    pub confidence: f64,
    pub implementation_steps: Vec<String>,
}

impl DynamicReplanner {
    /// Create a new dynamic replanner
    pub fn new(config: ReplannerConfig) -> Self {
        Self {
            config,
            current_plan: Arc::new(RwLock::new(ActivePlan::default())),
            execution_monitor: Arc::new(RwLock::new(ExecutionMonitor::default())),
            adaptation_engine: Arc::new(RwLock::new(AdaptationEngine::default())),
        }
    }

    /// Initialize with an execution plan
    pub async fn initialize_plan(&self, plan: ExecutionPlan) -> Result<()> {
        let mut active_plan = self.current_plan.write().await;
        active_plan.plan_id = Uuid::new_v4().to_string();
        active_plan.original_plan = Some(plan.clone());
        active_plan.current_schedule = plan.phases
            .into_iter()
            .flat_map(|phase| {
                let phase_id = phase.phase_id.clone();
                phase.tasks.into_iter().map(move |task_id| 
                    ScheduledTask {
                        task_id,
                        scheduled_start: Duration::from_secs(0),
                        estimated_end: Duration::from_secs(300),
                        execution_group: phase_id.clone(),
                        dependencies_resolved: true,
                        resource_allocation: Vec::new(),
                    }
                )
            })
            .collect();
        active_plan.plan_start_time = Some(SystemTime::now());
        
        // Initialize monitoring
        let mut monitor = self.execution_monitor.write().await;
        monitor.monitoring_start = Some(SystemTime::now());
        
        Ok(())
    }

    /// Update task progress and check for replanning needs
    pub async fn update_task_progress(&self, task_id: &str, result: &TaskResult) -> Result<Option<ReplanningResult>> {
        // Update progress tracking
        self.update_progress_tracking(task_id, result).await?;
        
        // Check if replanning is needed
        if self.should_replan().await? {
            let replan_result = self.execute_replanning().await?;
            return Ok(Some(replan_result));
        }
        
        Ok(None)
    }

    /// Update progress tracking for a task
    async fn update_progress_tracking(&self, task_id: &str, result: &TaskResult) -> Result<()> {
        let mut monitor = self.execution_monitor.write().await;
        
        let progress = TaskProgress {
            task_id: task_id.to_string(),
            status: if result.success {
                TaskExecutionStatus::Completed
            } else {
                TaskExecutionStatus::Failed
            },
            progress_percentage: 100.0,
            actual_start_time: Some(SystemTime::now() - result.execution_time),
            estimated_completion: None,
            actual_completion: Some(SystemTime::now()),
            performance_issues: if !result.success {
                vec![result.error_message.clone().unwrap_or_default()]
            } else {
                Vec::new()
            },
        };
        
        monitor.task_progress.insert(task_id.to_string(), progress);
        
        // Update plan state
        let mut active_plan = self.current_plan.write().await;
        if result.success {
            active_plan.completed_tasks.push(task_id.to_string());
            active_plan.in_progress_tasks.retain(|id| id != task_id);
        } else {
            active_plan.failed_tasks.push(task_id.to_string());
            active_plan.in_progress_tasks.retain(|id| id != task_id);
        }
        
        Ok(())
    }

    /// Determine if replanning is needed
    async fn should_replan(&self) -> Result<bool> {
        let monitor = self.execution_monitor.read().await;
        let active_plan = self.current_plan.read().await;
        
        // Check failure rate
        let total_completed = active_plan.completed_tasks.len() + active_plan.failed_tasks.len();
        if total_completed > 0 {
            let failure_rate = active_plan.failed_tasks.len() as f64 / total_completed as f64;
            if failure_rate > self.config.replan_threshold {
                return Ok(true);
            }
        }
        
        // Check schedule deviation
        let schedule_deviation = self.calculate_schedule_deviation(&active_plan, &monitor).await?;
        if schedule_deviation > self.config.replan_threshold {
            return Ok(true);
        }
        
        // Check time since last replan
        if let Some(last_replan) = active_plan.last_replan_time {
            let time_since_replan = SystemTime::now().duration_since(last_replan).unwrap_or_default();
            if time_since_replan.as_secs() < self.config.min_replan_interval {
                return Ok(false);
            }
        }
        
        // Check triggers
        for trigger in &monitor.condition_triggers {
            if trigger.current_value > trigger.threshold {
                return Ok(true);
            }
        }
        
        Ok(false)
    }

    /// Calculate schedule deviation
    async fn calculate_schedule_deviation(&self, plan: &ActivePlan, monitor: &ExecutionMonitor) -> Result<f64> {
        let mut total_deviation = 0.0;
        let mut task_count = 0;
        
        for task in &plan.current_schedule {
            if let Some(progress) = monitor.task_progress.get(&task.task_id) {
                if let (Some(actual_start), Some(actual_completion)) = 
                   (progress.actual_start_time, progress.actual_completion) {
                    let actual_duration = actual_completion.duration_since(actual_start).unwrap_or_default();
                    let expected_duration = task.estimated_end - task.scheduled_start;
                    
                    let deviation = (actual_duration.as_secs() as f64 - expected_duration.as_secs() as f64).abs() / 
                                   expected_duration.as_secs().max(1) as f64;
                    total_deviation += deviation;
                    task_count += 1;
                }
            }
        }
        
        if task_count > 0 {
            Ok(total_deviation / task_count as f64)
        } else {
            Ok(0.0)
        }
    }

    /// Execute replanning process
    async fn execute_replanning(&self) -> Result<ReplanningResult> {
        let adaptation_engine = self.adaptation_engine.read().await;
        let current_plan = self.current_plan.read().await;
        let monitor = self.execution_monitor.read().await;
        
        // Identify issues that need addressing
        let issues = self.identify_issues(&current_plan, &monitor).await?;
        
        // Select adaptation strategy
        let strategy = self.select_adaptation_strategy(&issues, &adaptation_engine).await?;
        
        // Generate new plan
        let new_schedule = self.generate_new_schedule(&current_plan, &strategy).await?;
        
        // Calculate expected improvement
        let expected_improvement = self.estimate_improvement(&strategy).await?;
        
        let modifications = vec![PlanModification {
            modification_id: Uuid::new_v4().to_string(),
            modification_type: ModificationType::TaskReorder,
            timestamp: SystemTime::now(),
            reason: format!("Adaptive replanning: {}", strategy.strategy_name),
            affected_tasks: new_schedule.iter().map(|s| s.task_id.clone()).collect(),
            performance_impact: expected_improvement,
        }];

        Ok(ReplanningResult {
            new_plan: ExecutionPlan {
                plan_id: Uuid::new_v4().to_string(),
                phases: vec![], // Would be constructed from new_schedule
                total_time: Duration::from_secs(1800),
                parallel_groups: Vec::new(),
            },
            modifications,
            rationale: format!("Applied strategy: {} to address performance issues", strategy.strategy_name),
            expected_improvement,
            confidence: 0.8,
            implementation_steps: vec![
                "Update task schedule".to_string(),
                "Reallocate resources".to_string(),
                "Notify execution engine".to_string(),
            ],
        })
    }

    /// Identify current issues in execution
    async fn identify_issues(&self, plan: &ActivePlan, monitor: &ExecutionMonitor) -> Result<Vec<String>> {
        let mut issues = Vec::new();
        
        // Check for failed tasks
        if !plan.failed_tasks.is_empty() {
            issues.push(format!("{} tasks have failed", plan.failed_tasks.len()));
        }
        
        // Check for resource contention
        for (resource_id, usage) in &monitor.resource_usage {
            if usage.current_utilization > 0.9 {
                issues.push(format!("High utilization on resource: {}", resource_id));
            }
        }
        
        // Check for schedule delays
        let schedule_deviation = self.calculate_schedule_deviation(plan, monitor).await?;
        if schedule_deviation > 0.2 {
            issues.push("Significant schedule deviation detected".to_string());
        }
        
        Ok(issues)
    }

    /// Select appropriate adaptation strategy
    async fn select_adaptation_strategy(
        &self,
        _issues: &[String],
        engine: &AdaptationEngine,
    ) -> Result<AdaptationStrategy> {
        // Select strategy with highest success rate
        if let Some(strategy) = engine.adaptation_strategies.iter()
            .max_by(|a, b| a.success_rate.partial_cmp(&b.success_rate).unwrap_or(std::cmp::Ordering::Equal)) {
            Ok(strategy.clone())
        } else {
            // Default strategy
            Ok(AdaptationStrategy {
                strategy_id: Uuid::new_v4().to_string(),
                strategy_name: "Default Reorder".to_string(),
                applicable_triggers: vec![TriggerType::PerformanceDegradation],
                adaptation_actions: vec![AdaptationAction {
                    action_type: ActionType::ReorderTasks,
                    parameters: HashMap::new(),
                    expected_impact: 0.2,
                    risk_level: 0.3,
                }],
                success_rate: 0.7,
                implementation_complexity: 0.5,
            })
        }
    }

    /// Generate new schedule based on adaptation strategy
    async fn generate_new_schedule(
        &self,
        current_plan: &ActivePlan,
        _strategy: &AdaptationStrategy,
    ) -> Result<Vec<ScheduledTask>> {
        // Simple implementation: reorder remaining tasks by priority
        let mut remaining_tasks = current_plan.pending_tasks.clone();
        remaining_tasks.sort(); // Simple alphabetical sort for now
        
        let mut new_schedule = Vec::new();
        let mut current_time = Duration::from_secs(0);
        
        for task_id in remaining_tasks {
            new_schedule.push(ScheduledTask {
                task_id: task_id.clone(),
                scheduled_start: current_time,
                estimated_end: current_time + Duration::from_secs(300),
                execution_group: "reordered".to_string(),
                dependencies_resolved: true,
                resource_allocation: Vec::new(),
            });
            current_time += Duration::from_secs(300);
        }
        
        Ok(new_schedule)
    }

    /// Estimate improvement from adaptation strategy
    async fn estimate_improvement(&self, strategy: &AdaptationStrategy) -> Result<f64> {
        // Average expected impact of all actions in the strategy
        let total_impact: f64 = strategy.adaptation_actions.iter()
            .map(|action| action.expected_impact)
            .sum();
        
        Ok(total_impact / strategy.adaptation_actions.len().max(1) as f64)
    }

    /// Apply replanning result to the current plan
    pub async fn apply_replan(&self, result: ReplanningResult) -> Result<()> {
        let mut active_plan = self.current_plan.write().await;
        
        // Update the current schedule
        active_plan.current_schedule.clear();
        // Would extract schedule from result.new_plan
        
        // Record modifications
        active_plan.plan_modifications.extend(result.modifications);
        active_plan.last_replan_time = Some(SystemTime::now());
        
        // Record adaptation event
        let mut adaptation_engine = self.adaptation_engine.write().await;
        adaptation_engine.recent_adaptations.push_back(AdaptationEvent {
            event_id: Uuid::new_v4().to_string(),
            timestamp: SystemTime::now(),
            trigger_reason: result.rationale,
            adaptation_applied: "Dynamic replan".to_string(),
            performance_before: 0.5, // Would calculate actual performance
            performance_after: None, // Will be updated later
            success: None, // Will be evaluated after implementation
        });
        
        Ok(())
    }

    /// Get current plan status
    pub async fn get_plan_status(&self) -> Result<PlanStatus> {
        let plan = self.current_plan.read().await;
        let monitor = self.execution_monitor.read().await;
        
        Ok(PlanStatus {
            plan_id: plan.plan_id.clone(),
            total_tasks: plan.current_schedule.len() as u32,
            completed_tasks: plan.completed_tasks.len() as u32,
            failed_tasks: plan.failed_tasks.len() as u32,
            in_progress_tasks: plan.in_progress_tasks.len() as u32,
            pending_tasks: plan.pending_tasks.len() as u32,
            modifications_count: plan.plan_modifications.len() as u32,
            current_performance: plan.plan_performance.clone(),
            next_scheduled_task: plan.current_schedule.first().map(|s| s.task_id.clone()),
        })
    }
}

/// Current status of the execution plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStatus {
    pub plan_id: String,
    pub total_tasks: u32,
    pub completed_tasks: u32,
    pub failed_tasks: u32,
    pub in_progress_tasks: u32,
    pub pending_tasks: u32,
    pub modifications_count: u32,
    pub current_performance: PlanPerformance,
    pub next_scheduled_task: Option<String>,
}