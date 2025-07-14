//! Learning insights and knowledge extraction components
//! 
//! This module handles the extraction of learning insights from reflection analysis,
//! pattern recognition, and knowledge retention strategies.

use anyhow::Result;
use std::collections::HashMap;

use crate::context::ExecutionContext;
use crate::reflection::types::*;

/// Learning insight extractor
pub struct LearningInsightExtractor;

impl LearningInsightExtractor {
    /// Extract learning insights from reflection analysis
    pub async fn extract_insights(
        analysis: &ReflectionAnalysis,
        context: &ExecutionContext,
    ) -> Result<Vec<LearningInsight>> {
        let mut insights = Vec::new();

        // Extract insights from success patterns
        let success_insights = Self::extract_success_insights(&analysis.success_patterns).await?;
        insights.extend(success_insights);

        // Extract insights from failure patterns
        let failure_insights = Self::extract_failure_insights(&analysis.failure_patterns).await?;
        insights.extend(failure_insights);

        // Extract insights from strategy effectiveness
        let strategy_insights = Self::extract_strategy_insights(&analysis.strategy_effectiveness).await?;
        insights.extend(strategy_insights);

        // Extract insights from resource utilization
        let resource_insights = Self::extract_resource_insights(&analysis.resource_utilization).await?;
        insights.extend(resource_insights);

        // Extract contextual insights
        let contextual_insights = Self::extract_contextual_insights(context).await?;
        insights.extend(contextual_insights);

        Ok(insights)
    }

    /// Extract insights from success patterns
    async fn extract_success_insights(patterns: &[SuccessPattern]) -> Result<Vec<LearningInsight>> {
        let mut insights = Vec::new();

        for pattern in patterns {
            insights.push(LearningInsight {
                insight_id: uuid::Uuid::new_v4().to_string(),
                insight_type: InsightType::SuccessFactors,
                description: format!("Success pattern identified: {}", pattern.description),
                evidence: pattern.conditions.clone(),
                confidence: pattern.success_rate,
                applicability: Applicability {
                    context_types: vec!["similar_goals".to_string(), "comparable_complexity".to_string()],
                    goal_types: vec!["analysis".to_string(), "problem_solving".to_string()],
                    confidence_level: pattern.success_rate,
                    generalizability: Self::calculate_generalizability(pattern.success_rate),
                },
                retention_value: pattern.success_rate,
            });

            // Extract specific technique insights
            if pattern.success_rate > 0.8 {
                insights.push(LearningInsight {
                    insight_id: uuid::Uuid::new_v4().to_string(),
                    insight_type: InsightType::TechniqueOptimization,
                    description: format!("High-success technique: {}", pattern.description),
                    evidence: pattern.conditions.clone(),
                    confidence: pattern.success_rate,
                    applicability: Applicability {
                        context_types: vec!["high_stakes".to_string(), "critical_tasks".to_string()],
                        goal_types: vec!["optimization".to_string(), "performance".to_string()],
                        confidence_level: pattern.success_rate,
                        generalizability: 0.9,
                    },
                    retention_value: pattern.success_rate * 1.2, // Higher retention for high-success patterns
                });
            }
        }

        Ok(insights)
    }

    /// Extract insights from failure patterns
    async fn extract_failure_insights(patterns: &[FailurePattern]) -> Result<Vec<LearningInsight>> {
        let mut insights = Vec::new();

        for pattern in patterns {
            insights.push(LearningInsight {
                insight_id: uuid::Uuid::new_v4().to_string(),
                insight_type: InsightType::FailureFactors,
                description: format!("Failure pattern identified: {}", pattern.description),
                evidence: pattern.conditions.clone(),
                confidence: pattern.failure_rate,
                applicability: Applicability {
                    context_types: vec!["similar_goals".to_string(), "risk_prone_tasks".to_string()],
                    goal_types: vec!["analysis".to_string(), "problem_solving".to_string()],
                    confidence_level: pattern.failure_rate,
                    generalizability: Self::calculate_generalizability(pattern.failure_rate),
                },
                retention_value: pattern.failure_rate * 0.8, // Slightly lower retention for failure patterns
            });

            // Extract avoidance strategies
            if pattern.failure_rate > 0.6 {
                insights.push(LearningInsight {
                    insight_id: uuid::Uuid::new_v4().to_string(),
                    insight_type: InsightType::AvoidanceStrategies,
                    description: format!("Avoid: {}", pattern.description),
                    evidence: pattern.conditions.clone(),
                    confidence: pattern.failure_rate,
                    applicability: Applicability {
                        context_types: vec!["preventive_measures".to_string()],
                        goal_types: vec!["risk_mitigation".to_string()],
                        confidence_level: pattern.failure_rate,
                        generalizability: 0.7,
                    },
                    retention_value: pattern.failure_rate,
                });
            }
        }

        Ok(insights)
    }

    /// Extract insights from strategy effectiveness
    async fn extract_strategy_insights(effectiveness: &StrategyEffectiveness) -> Result<Vec<LearningInsight>> {
        let mut insights = Vec::new();

        if effectiveness.current_strategy_score > 0.8 {
            insights.push(LearningInsight {
                insight_id: uuid::Uuid::new_v4().to_string(),
                insight_type: InsightType::StrategyOptimization,
                description: "Current strategy is highly effective".to_string(),
                evidence: vec![format!("Strategy score: {:.2}", effectiveness.current_strategy_score)],
                confidence: effectiveness.current_strategy_score,
                applicability: Applicability {
                    context_types: vec!["similar_contexts".to_string()],
                    goal_types: vec!["strategic_planning".to_string()],
                    confidence_level: effectiveness.current_strategy_score,
                    generalizability: 0.8,
                },
                retention_value: effectiveness.current_strategy_score,
            });
        }

        if effectiveness.strategy_consistency < 0.5 {
            insights.push(LearningInsight {
                insight_id: uuid::Uuid::new_v4().to_string(),
                insight_type: InsightType::ConsistencyImprovement,
                description: "Strategy consistency needs improvement".to_string(),
                evidence: vec![format!("Consistency score: {:.2}", effectiveness.strategy_consistency)],
                confidence: 1.0 - effectiveness.strategy_consistency,
                applicability: Applicability {
                    context_types: vec!["strategy_planning".to_string()],
                    goal_types: vec!["consistency".to_string()],
                    confidence_level: 0.8,
                    generalizability: 0.7,
                },
                retention_value: 0.8,
            });
        }

        Ok(insights)
    }

    /// Extract insights from resource utilization
    async fn extract_resource_insights(utilization: &ResourceUtilization) -> Result<Vec<LearningInsight>> {
        let mut insights = Vec::new();

        if utilization.efficiency_score < 0.6 {
            insights.push(LearningInsight {
                insight_id: uuid::Uuid::new_v4().to_string(),
                insight_type: InsightType::ResourceOptimization,
                description: "Resource utilization efficiency is below optimal".to_string(),
                evidence: vec![
                    format!("Efficiency score: {:.2}", utilization.efficiency_score),
                    format!("CPU utilization: {:.2}", utilization.cpu_utilization),
                    format!("Memory utilization: {:.2}", utilization.memory_utilization),
                ],
                confidence: 1.0 - utilization.efficiency_score,
                applicability: Applicability {
                    context_types: vec!["resource_constrained".to_string()],
                    goal_types: vec!["optimization".to_string()],
                    confidence_level: 0.8,
                    generalizability: 0.9,
                },
                retention_value: 0.7,
            });
        }

        Ok(insights)
    }

    /// Extract contextual insights
    async fn extract_contextual_insights(context: &ExecutionContext) -> Result<Vec<LearningInsight>> {
        let mut insights = Vec::new();

        // Analyze task completion patterns
        let completion_rate = if !context.completed_tasks.is_empty() {
            context.completed_tasks
                .iter()
                .filter(|task| task.success == Some(true))
                .count() as f64 / context.completed_tasks.len() as f64
        } else {
            0.0
        };

        if completion_rate > 0.8 {
            insights.push(LearningInsight {
                insight_id: uuid::Uuid::new_v4().to_string(),
                insight_type: InsightType::ContextualFactors,
                description: "High task completion rate in current context".to_string(),
                evidence: vec![format!("Completion rate: {:.2}", completion_rate)],
                confidence: completion_rate,
                applicability: Applicability {
                    context_types: vec!["similar_task_types".to_string()],
                    goal_types: vec!["task_execution".to_string()],
                    confidence_level: completion_rate,
                    generalizability: 0.7,
                },
                retention_value: completion_rate,
            });
        }

        Ok(insights)
    }

    /// Calculate generalizability score
    fn calculate_generalizability(confidence: f64) -> f64 {
        // Higher confidence patterns are more generalizable
        (confidence * 0.8 + 0.2).min(1.0)
    }
}

/// Pattern recognition engine
pub struct PatternRecognizer;

impl PatternRecognizer {
    /// Recognize success patterns from execution history
    pub async fn recognize_success_patterns(context: &ExecutionContext) -> Result<Vec<SuccessPattern>> {
        let mut patterns = Vec::new();

        // Analyze successful tasks
        let successful_tasks: Vec<_> = context.completed_tasks
            .iter()
            .filter(|task| task.success == Some(true))
            .collect();

        if !successful_tasks.is_empty() {
            // Group by similar characteristics
            let pattern_groups = Self::group_tasks_by_similarity(&successful_tasks);

            for (pattern_key, tasks) in pattern_groups {
                let success_rate = tasks.len() as f64 / context.completed_tasks.len() as f64;
                
                if success_rate > 0.3 { // Only consider patterns with reasonable frequency
                    patterns.push(SuccessPattern {
                        pattern_id: uuid::Uuid::new_v4().to_string(),
                        description: format!("Success pattern: {}", pattern_key),
                        conditions: vec![pattern_key.clone()],
                        actions: vec!["systematic_execution".to_string(), "validation".to_string()],
                        success_rate,
                        frequency: tasks.len(),
                        context_factors: Self::extract_context_factors(&tasks),
                    });
                }
            }
        }

        Ok(patterns)
    }

    /// Recognize failure patterns from execution history
    pub async fn recognize_failure_patterns(context: &ExecutionContext) -> Result<Vec<FailurePattern>> {
        let mut patterns = Vec::new();

        // Analyze failed tasks
        let failed_tasks: Vec<_> = context.completed_tasks
            .iter()
            .filter(|task| task.success == Some(false))
            .collect();

        if !failed_tasks.is_empty() {
            // Group by similar characteristics
            let pattern_groups = Self::group_tasks_by_similarity(&failed_tasks);

            for (pattern_key, tasks) in pattern_groups {
                let failure_rate = tasks.len() as f64 / context.completed_tasks.len() as f64;
                
                if failure_rate > 0.2 { // Consider patterns with reasonable frequency
                    patterns.push(FailurePattern {
                        pattern_id: uuid::Uuid::new_v4().to_string(),
                        description: format!("Failure pattern: {}", pattern_key),
                        conditions: vec![pattern_key.clone()],
                        actions: vec!["rushed_execution".to_string(), "insufficient_validation".to_string()],
                        failure_rate,
                        mitigation_strategies: vec!["improve_planning".to_string(), "enhance_validation".to_string()],
                        frequency: tasks.len(),
                        context_factors: Self::extract_context_factors(&tasks),
                    });
                }
            }
        }

        Ok(patterns)
    }

    /// Group tasks by similarity
    fn group_tasks_by_similarity<'a>(tasks: &'a [&'a crate::task::Task]) -> HashMap<String, Vec<&'a crate::task::Task>> {
        let mut groups = HashMap::new();

        for task in tasks {
            // Simple grouping by task type or description patterns
            let key = if task.description.contains("analysis") {
                "analysis_tasks".to_string()
            } else if task.description.contains("file") {
                "file_operations".to_string()
            } else if task.description.contains("process") {
                "processing_tasks".to_string()
            } else {
                "general_tasks".to_string()
            };

            groups.entry(key).or_insert_with(Vec::new).push(*task);
        }

        groups
    }

    /// Extract context factors from tasks
    fn extract_context_factors(tasks: &[&crate::task::Task]) -> Vec<String> {
        let mut factors = Vec::new();

        // Extract common characteristics
        if tasks.len() > 1 {
            factors.push("multiple_similar_tasks".to_string());
        }

        // Add more sophisticated factor extraction here
        factors.push("task_complexity_medium".to_string());

        factors
    }
}

/// Knowledge retention manager
pub struct KnowledgeRetentionManager;

impl KnowledgeRetentionManager {
    /// Prioritize insights for retention
    pub async fn prioritize_for_retention(insights: &[LearningInsight]) -> Result<Vec<RetentionPriority>> {
        let mut priorities = Vec::new();

        for insight in insights {
            let priority_score = Self::calculate_retention_priority(insight);
            
            priorities.push(RetentionPriority {
                insight_id: insight.insight_id.clone(),
                priority_score,
                retention_strategy: Self::determine_retention_strategy(insight),
                review_frequency: Self::calculate_review_frequency(insight),
            });
        }

        // Sort by priority score (highest first)
        priorities.sort_by(|a, b| {
            b.priority_score.partial_cmp(&a.priority_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(priorities)
    }

    /// Calculate retention priority score
    fn calculate_retention_priority(insight: &LearningInsight) -> f64 {
        let confidence_weight = insight.confidence * 0.4;
        let applicability_weight = insight.applicability.generalizability * 0.3;
        let retention_weight = insight.retention_value * 0.3;

        confidence_weight + applicability_weight + retention_weight
    }

    /// Determine retention strategy
    fn determine_retention_strategy(insight: &LearningInsight) -> RetentionStrategy {
        match insight.insight_type {
            InsightType::SuccessFactors => RetentionStrategy::ReinforcementLearning,
            InsightType::FailureFactors => RetentionStrategy::AvoidancePattern,
            InsightType::TechniqueOptimization => RetentionStrategy::BestPractice,
            InsightType::StrategyOptimization => RetentionStrategy::StrategicKnowledge,
            InsightType::ResourceOptimization => RetentionStrategy::OperationalKnowledge,
            InsightType::ContextualFactors => RetentionStrategy::ContextualMemory,
            InsightType::AvoidanceStrategies => RetentionStrategy::AvoidancePattern,
            InsightType::ConsistencyImprovement => RetentionStrategy::ProcessImprovement,
            InsightType::CausalRelationship => RetentionStrategy::OperationalKnowledge,
            InsightType::PerformancePattern => RetentionStrategy::BestPractice,
            InsightType::EnvironmentalInfluence => RetentionStrategy::ContextualMemory,
            InsightType::ToolEffectiveness => RetentionStrategy::OperationalKnowledge,
        }
    }

    /// Calculate review frequency
    fn calculate_review_frequency(insight: &LearningInsight) -> std::time::Duration {
        let base_frequency = std::time::Duration::from_secs(3600); // 1 hour

        // Adjust based on retention value
        let multiplier = if insight.retention_value > 0.8 {
            0.5 // More frequent review for high-value insights
        } else if insight.retention_value > 0.6 {
            1.0
        } else {
            2.0 // Less frequent review for lower-value insights
        };

        std::time::Duration::from_secs((base_frequency.as_secs() as f64 * multiplier) as u64)
    }
}
