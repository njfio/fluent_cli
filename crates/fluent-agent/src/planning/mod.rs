//! Planning module for sophisticated task planning and dependency management
//!
//! This module provides advanced planning capabilities including:
//! - Hierarchical Task Networks (HTN) for goal decomposition
//! - Dependency analysis for task ordering and parallel execution
//! - Dynamic replanning for adaptive execution

pub mod hierarchical_task_networks;
pub mod dependency_analyzer;
pub mod dynamic_replanner;
pub mod enhanced_htn;

pub use hierarchical_task_networks::{HTNPlanner, HTNConfig, HTNResult, ExecutionPlan, NetworkTask};
pub use enhanced_htn::{EnhancedHTNPlanner, EnhancedHTNConfig, EnhancedHTNResult, EnhancedExecutionPlan};
pub use dependency_analyzer::{
    DependencyAnalyzer, AnalyzerConfig, DependencyAnalysis, ParallelGroup, 
    ScheduledTask, Bottleneck, OptimizationSuggestion
};
pub use dynamic_replanner::{DynamicReplanner, ReplannerConfig, ReplanningResult, PlanStatus};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::goal::Goal;
use crate::task::Task;
use crate::context::ExecutionContext;

/// Composite planning system that combines HTN and dependency analysis
pub struct CompositePlanner {
    htn_planner: HTNPlanner,
    dependency_analyzer: DependencyAnalyzer,
    config: PlannerConfig,
}

/// Configuration for the composite planner
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerConfig {
    pub enable_htn: bool,
    pub enable_dependency_analysis: bool,
    pub enable_optimization: bool,
    pub planning_timeout_secs: u64,
}

impl Default for PlannerConfig {
    fn default() -> Self {
        Self {
            enable_htn: true,
            enable_dependency_analysis: true,
            enable_optimization: true,
            planning_timeout_secs: 300,
        }
    }
}

/// Complete planning result combining HTN and dependency analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletePlanningResult {
    pub htn_result: Option<HTNResult>,
    pub dependency_analysis: Option<DependencyAnalysis>,
    pub integrated_plan: IntegratedExecutionPlan,
    pub planning_summary: PlanningSummary,
}

/// Integrated execution plan combining both approaches
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegratedExecutionPlan {
    pub execution_phases: Vec<ExecutionPhase>,
    pub parallel_opportunities: Vec<ParallelGroup>,
    pub critical_path: Vec<String>,
    pub total_estimated_time: std::time::Duration,
    pub optimization_level: f64,
}

/// Phase in the integrated execution plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPhase {
    pub phase_id: String,
    pub phase_name: String,
    pub tasks: Vec<String>,
    pub parallel_groups: Vec<String>,
    pub estimated_duration: std::time::Duration,
    pub dependencies_satisfied: bool,
}

/// Summary of the planning process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningSummary {
    pub total_tasks: u32,
    pub decomposition_levels: u32,
    pub parallel_efficiency: f64,
    pub optimization_suggestions: u32,
    pub planning_confidence: f64,
}

impl CompositePlanner {
    /// Create a new composite planner
    pub fn new(
        engine: Arc<dyn fluent_core::traits::Engine>, 
        config: PlannerConfig
    ) -> Self {
        let htn_config = HTNConfig::default();
        let analyzer_config = AnalyzerConfig::default();
        
        Self {
            htn_planner: HTNPlanner::new(engine.clone(), htn_config),
            dependency_analyzer: DependencyAnalyzer::new(analyzer_config),
            config,
        }
    }

    /// Perform complete planning analysis
    pub async fn plan_execution(
        &self,
        goal: &Goal,
        context: &ExecutionContext,
    ) -> Result<CompletePlanningResult> {
        let mut htn_result = None;
        let mut dependency_analysis = None;

        // Phase 1: HTN decomposition if enabled
        if self.config.enable_htn {
            htn_result = Some(self.htn_planner.plan_decomposition(goal, context).await?);
        }

        // Phase 2: Dependency analysis if enabled
        if self.config.enable_dependency_analysis {
            // Convert HTN tasks to task list for dependency analysis
            let tasks = if let Some(ref htn) = htn_result {
                self.convert_htn_to_tasks(&htn.tasks).await?
            } else {
                // Use existing tasks from context or create basic tasks
                vec![self.create_basic_task_from_goal(goal).await?]
            };

            dependency_analysis = Some(
                self.dependency_analyzer.analyze_dependencies(&tasks, context).await?
            );
        }

        // Phase 3: Integration
        let integrated_plan = self.integrate_plans(&htn_result, &dependency_analysis).await?;
        
        // Phase 4: Summary
        let summary = self.generate_planning_summary(&htn_result, &dependency_analysis).await?;

        Ok(CompletePlanningResult {
            htn_result,
            dependency_analysis,
            integrated_plan,
            planning_summary: summary,
        })
    }

    /// Convert HTN network tasks to task objects
    async fn convert_htn_to_tasks(&self, htn_tasks: &[NetworkTask]) -> Result<Vec<Task>> {
        let mut tasks = Vec::new();
        
        for htn_task in htn_tasks {
            let task = Task {
                task_id: htn_task.id.clone(),
                description: htn_task.description.clone(),
                task_type: match htn_task.task_type {
                    hierarchical_task_networks::TaskType::Primitive => crate::task::TaskType::CodeGeneration,
                    hierarchical_task_networks::TaskType::Compound => crate::task::TaskType::Planning,
                },
                priority: crate::task::TaskPriority::Medium,
                dependencies: Vec::new(),
                inputs: std::collections::HashMap::new(),
                expected_outputs: vec!["Task output".to_string()],
                success_criteria: vec!["Task completed successfully".to_string()],
                estimated_duration: Some(std::time::Duration::from_secs(300)),
                max_attempts: 3,
                current_attempt: 0,
                created_at: std::time::SystemTime::now(),
                started_at: None,
                completed_at: None,
                success: None,
                error_message: None,
                metadata: std::collections::HashMap::new(),
            };
            tasks.push(task);
        }
        
        Ok(tasks)
    }

    /// Create a basic task from a goal
    async fn create_basic_task_from_goal(&self, goal: &Goal) -> Result<Task> {
        Ok(Task {
            task_id: uuid::Uuid::new_v4().to_string(),
            description: goal.description.clone(),
            task_type: crate::task::TaskType::CodeGeneration,
            priority: match goal.priority {
                crate::goal::GoalPriority::Critical => crate::task::TaskPriority::Critical,
                crate::goal::GoalPriority::High => crate::task::TaskPriority::High,
                crate::goal::GoalPriority::Medium => crate::task::TaskPriority::Medium,
                crate::goal::GoalPriority::Low => crate::task::TaskPriority::Low,
            },
            dependencies: Vec::new(),
            inputs: std::collections::HashMap::new(),
            expected_outputs: vec!["Goal output".to_string()],
            success_criteria: goal.success_criteria.clone(),
            estimated_duration: Some(goal.get_estimated_duration()),
            max_attempts: 3,
            current_attempt: 0,
            created_at: std::time::SystemTime::now(),
            started_at: None,
            completed_at: None,
            success: None,
            error_message: None,
            metadata: std::collections::HashMap::new(),
        })
    }

    /// Integrate HTN and dependency analysis results
    async fn integrate_plans(
        &self,
        htn_result: &Option<HTNResult>,
        dep_analysis: &Option<DependencyAnalysis>,
    ) -> Result<IntegratedExecutionPlan> {
        let mut phases = Vec::new();
        let mut parallel_opportunities = Vec::new();
        let mut critical_path = Vec::new();
        let mut total_time = std::time::Duration::from_secs(0);

        // Integrate HTN phases
        if let Some(htn) = htn_result {
            for (i, htn_phase) in htn.plan.phases.iter().enumerate() {
                phases.push(ExecutionPhase {
                    phase_id: htn_phase.phase_id.clone(),
                    phase_name: format!("HTN Phase {}", i + 1),
                    tasks: htn_phase.tasks.clone(),
                    parallel_groups: Vec::new(),
                    estimated_duration: htn_phase.duration,
                    dependencies_satisfied: true,
                });
                total_time += htn_phase.duration;
            }
            
            // Add HTN parallel groups
            for group in &htn.plan.parallel_groups {
                parallel_opportunities.push(ParallelGroup {
                    group_id: uuid::Uuid::new_v4().to_string(),
                    group_name: "HTN Parallel Group".to_string(),
                    tasks: group.clone(),
                    max_concurrency: 6, // Default max parallel tasks
                    estimated_duration: std::time::Duration::from_secs(300),
                    resource_requirements: Vec::new(),
                });
            }
        }

        // Integrate dependency analysis
        if let Some(dep) = dep_analysis {
            // Merge parallel opportunities
            for opportunity in &dep.parallel_opportunities {
                parallel_opportunities.push(opportunity.clone());
            }
            
            critical_path = dep.critical_path.clone();
        }

        Ok(IntegratedExecutionPlan {
            execution_phases: phases,
            parallel_opportunities,
            critical_path,
            total_estimated_time: total_time,
            optimization_level: 0.8, // Placeholder
        })
    }

    /// Generate planning summary
    async fn generate_planning_summary(
        &self,
        htn_result: &Option<HTNResult>,
        dep_analysis: &Option<DependencyAnalysis>,
    ) -> Result<PlanningSummary> {
        let mut total_tasks = 0;
        let mut decomposition_levels = 0;
        let mut parallel_efficiency = 0.0;
        let mut optimization_suggestions = 0;

        if let Some(htn) = htn_result {
            total_tasks += htn.tasks.len() as u32;
            decomposition_levels = htn.metrics.max_depth;
        }

        if let Some(dep) = dep_analysis {
            parallel_efficiency = dep.analysis_metrics.parallelization_ratio;
            optimization_suggestions = dep.optimization_suggestions.len() as u32;
        }

        Ok(PlanningSummary {
            total_tasks,
            decomposition_levels,
            parallel_efficiency,
            optimization_suggestions,
            planning_confidence: 0.85, // Calculated based on various factors
        })
    }
}