//! Recommendation generation and prioritization components
//! 
//! This module handles the generation of actionable recommendations based on
//! reflection analysis, strategy adjustments, and learning insights.

use anyhow::Result;
use std::time::Duration;

use crate::reflection::types::*;

/// Recommendation generator
pub struct RecommendationGenerator;

impl RecommendationGenerator {
    /// Generate comprehensive recommendations
    pub async fn generate_recommendations(
        analysis: &ReflectionAnalysis,
        strategy_adjustments: &[StrategyAdjustment],
        learning_insights: &[LearningInsight],
    ) -> Result<Vec<Recommendation>> {
        let mut recommendations = Vec::new();

        // Generate immediate action recommendations
        let immediate_recommendations = Self::generate_immediate_actions(strategy_adjustments).await?;
        recommendations.extend(immediate_recommendations);

        // Generate learning recommendations
        let learning_recommendations = Self::generate_learning_recommendations(&analysis.learning_opportunities).await?;
        recommendations.extend(learning_recommendations);

        // Generate optimization recommendations
        let optimization_recommendations = Self::generate_optimization_recommendations(analysis).await?;
        recommendations.extend(optimization_recommendations);

        // Generate preventive recommendations
        let preventive_recommendations = Self::generate_preventive_recommendations(analysis).await?;
        recommendations.extend(preventive_recommendations);

        // Generate strategic recommendations
        let strategic_recommendations = Self::generate_strategic_recommendations(learning_insights).await?;
        recommendations.extend(strategic_recommendations);

        // Sort by priority and urgency
        recommendations.sort_by(|a, b| {
            let a_score = Self::calculate_recommendation_score(a);
            let b_score = Self::calculate_recommendation_score(b);
            b_score.partial_cmp(&a_score).unwrap()
        });

        Ok(recommendations)
    }

    /// Generate immediate action recommendations
    async fn generate_immediate_actions(strategy_adjustments: &[StrategyAdjustment]) -> Result<Vec<Recommendation>> {
        let mut recommendations = Vec::new();

        for adjustment in strategy_adjustments {
            if adjustment.expected_impact == ImpactLevel::Critical {
                recommendations.push(Recommendation {
                    recommendation_id: uuid::Uuid::new_v4().to_string(),
                    recommendation_type: RecommendationType::ImmediateAction,
                    description: format!("Implement critical adjustment: {}", adjustment.description),
                    priority: Priority::Critical,
                    urgency: Urgency::Immediate,
                    implementation_timeline: Duration::from_secs(300), // 5 minutes
                    expected_benefits: vec![
                        "Prevent further degradation".to_string(),
                        "Restore system stability".to_string(),
                        "Minimize impact".to_string(),
                    ],
                    potential_risks: vec![
                        "Disruption to current workflow".to_string(),
                        "Temporary performance impact".to_string(),
                    ],
                });
            } else if adjustment.expected_impact == ImpactLevel::High {
                recommendations.push(Recommendation {
                    recommendation_id: uuid::Uuid::new_v4().to_string(),
                    recommendation_type: RecommendationType::PerformanceImprovement,
                    description: format!("Implement high-impact adjustment: {}", adjustment.description),
                    priority: Priority::High,
                    urgency: Urgency::High,
                    implementation_timeline: Duration::from_secs(900), // 15 minutes
                    expected_benefits: vec![
                        "Significant performance improvement".to_string(),
                        "Enhanced efficiency".to_string(),
                    ],
                    potential_risks: vec!["Implementation complexity".to_string()],
                });
            }
        }

        Ok(recommendations)
    }

    /// Generate learning recommendations
    async fn generate_learning_recommendations(opportunities: &[LearningOpportunity]) -> Result<Vec<Recommendation>> {
        let mut recommendations = Vec::new();

        for opportunity in opportunities {
            if opportunity.potential_impact == ImpactLevel::High && opportunity.priority == Priority::High {
                recommendations.push(Recommendation {
                    recommendation_id: uuid::Uuid::new_v4().to_string(),
                    recommendation_type: RecommendationType::SkillDevelopment,
                    description: format!("Pursue learning opportunity: {}", opportunity.description),
                    priority: opportunity.priority.clone(),
                    urgency: Self::map_priority_to_urgency(&opportunity.priority),
                    implementation_timeline: Self::calculate_learning_timeline(&opportunity.implementation_difficulty),
                    expected_benefits: vec![
                        "Improved capabilities".to_string(),
                        "Better performance".to_string(),
                        "Enhanced problem-solving".to_string(),
                    ],
                    potential_risks: vec![
                        "Time investment required".to_string(),
                        "Learning curve impact".to_string(),
                    ],
                });
            }
        }

        Ok(recommendations)
    }

    /// Generate optimization recommendations
    async fn generate_optimization_recommendations(analysis: &ReflectionAnalysis) -> Result<Vec<Recommendation>> {
        let mut recommendations = Vec::new();

        // Resource optimization
        if analysis.resource_utilization.efficiency_score < 0.6 {
            recommendations.push(Recommendation {
                recommendation_id: uuid::Uuid::new_v4().to_string(),
                recommendation_type: RecommendationType::ResourceOptimization,
                description: "Optimize resource utilization to improve efficiency".to_string(),
                priority: Priority::Medium,
                urgency: Urgency::Medium,
                implementation_timeline: Duration::from_secs(1800), // 30 minutes
                expected_benefits: vec![
                    "Improved resource efficiency".to_string(),
                    "Better performance".to_string(),
                    "Reduced waste".to_string(),
                ],
                potential_risks: vec!["Temporary disruption".to_string()],
            });
        }

        // Strategy optimization
        if analysis.strategy_effectiveness.current_strategy_score < 0.7 {
            recommendations.push(Recommendation {
                recommendation_id: uuid::Uuid::new_v4().to_string(),
                recommendation_type: RecommendationType::StrategyRefinement,
                description: "Refine strategy to improve effectiveness".to_string(),
                priority: Priority::High,
                urgency: Urgency::Medium,
                implementation_timeline: Duration::from_secs(2700), // 45 minutes
                expected_benefits: vec![
                    "Improved strategy effectiveness".to_string(),
                    "Better goal alignment".to_string(),
                    "Enhanced decision-making".to_string(),
                ],
                potential_risks: vec![
                    "Strategy transition period".to_string(),
                    "Potential inconsistency".to_string(),
                ],
            });
        }

        Ok(recommendations)
    }

    /// Generate preventive recommendations
    async fn generate_preventive_recommendations(analysis: &ReflectionAnalysis) -> Result<Vec<Recommendation>> {
        let mut recommendations = Vec::new();

        // Bottleneck prevention
        for bottleneck in &analysis.bottlenecks_identified {
            if bottleneck.frequency > 0.3 {
                recommendations.push(Recommendation {
                    recommendation_id: uuid::Uuid::new_v4().to_string(),
                    recommendation_type: RecommendationType::PreventiveMeasure,
                    description: format!("Implement preventive measures for: {}", bottleneck.description),
                    priority: Priority::Medium,
                    urgency: Urgency::Low,
                    implementation_timeline: Duration::from_secs(3600), // 1 hour
                    expected_benefits: vec![
                        "Reduced bottleneck occurrence".to_string(),
                        "Improved reliability".to_string(),
                        "Better predictability".to_string(),
                    ],
                    potential_risks: vec!["Over-engineering".to_string()],
                });
            }
        }

        // Failure pattern prevention
        for pattern in &analysis.failure_patterns {
            if pattern.failure_rate > 0.4 {
                recommendations.push(Recommendation {
                    recommendation_id: uuid::Uuid::new_v4().to_string(),
                    recommendation_type: RecommendationType::RiskMitigation,
                    description: format!("Mitigate failure pattern: {}", pattern.description),
                    priority: Priority::Medium,
                    urgency: Urgency::Medium,
                    implementation_timeline: Duration::from_secs(2400), // 40 minutes
                    expected_benefits: vec![
                        "Reduced failure rate".to_string(),
                        "Improved reliability".to_string(),
                        "Better error handling".to_string(),
                    ],
                    potential_risks: vec!["Increased complexity".to_string()],
                });
            }
        }

        Ok(recommendations)
    }

    /// Generate strategic recommendations
    async fn generate_strategic_recommendations(learning_insights: &[LearningInsight]) -> Result<Vec<Recommendation>> {
        let mut recommendations = Vec::new();

        // High-value insights
        for insight in learning_insights {
            if insight.retention_value > 0.8 && insight.confidence > 0.7 {
                recommendations.push(Recommendation {
                    recommendation_id: uuid::Uuid::new_v4().to_string(),
                    recommendation_type: RecommendationType::StrategicAlignment,
                    description: format!("Leverage high-value insight: {}", insight.description),
                    priority: Priority::Medium,
                    urgency: Urgency::Low,
                    implementation_timeline: Duration::from_secs(1800), // 30 minutes
                    expected_benefits: vec![
                        "Strategic advantage".to_string(),
                        "Improved decision-making".to_string(),
                        "Better outcomes".to_string(),
                    ],
                    potential_risks: vec!["Over-reliance on patterns".to_string()],
                });
            }
        }

        Ok(recommendations)
    }

    /// Calculate recommendation priority score
    fn calculate_recommendation_score(recommendation: &Recommendation) -> f64 {
        let priority_weight = match recommendation.priority {
            Priority::Critical => 1.0,
            Priority::High => 0.8,
            Priority::Medium => 0.6,
            Priority::Low => 0.4,
        };

        let urgency_weight = match recommendation.urgency {
            Urgency::Immediate => 1.0,
            Urgency::High => 0.8,
            Urgency::Medium => 0.6,
            Urgency::Low => 0.4,
        };

        let type_weight = match recommendation.recommendation_type {
            RecommendationType::ImmediateAction => 1.0,
            RecommendationType::PerformanceImprovement => 0.9,
            RecommendationType::RiskMitigation => 0.8,
            RecommendationType::ResourceOptimization => 0.7,
            RecommendationType::SkillDevelopment => 0.6,
            RecommendationType::StrategyRefinement => 0.7,
            RecommendationType::PreventiveMeasure => 0.5,
            RecommendationType::StrategicAlignment => 0.6,
            RecommendationType::StrategyChange => 0.8,
            RecommendationType::ToolAdoption => 0.6,
            RecommendationType::ProcessImprovement => 0.7,
            RecommendationType::GoalAdjustment => 0.6,
            RecommendationType::ResourceRequest => 0.5,
        };

        (priority_weight * 0.4) + (urgency_weight * 0.4) + (type_weight * 0.2)
    }

    /// Map priority to urgency
    fn map_priority_to_urgency(priority: &Priority) -> Urgency {
        match priority {
            Priority::Critical => Urgency::Immediate,
            Priority::High => Urgency::High,
            Priority::Medium => Urgency::Medium,
            Priority::Low => Urgency::Low,
        }
    }

    /// Calculate learning timeline based on difficulty
    fn calculate_learning_timeline(difficulty: &DifficultyLevel) -> Duration {
        match difficulty {
            DifficultyLevel::Low => Duration::from_secs(1800), // 30 minutes
            DifficultyLevel::Easy => Duration::from_secs(1800), // 30 minutes
            DifficultyLevel::Medium => Duration::from_secs(3600), // 1 hour
            DifficultyLevel::Hard => Duration::from_secs(7200), // 2 hours
            DifficultyLevel::High => Duration::from_secs(7200), // 2 hours
            DifficultyLevel::Expert => Duration::from_secs(14400), // 4 hours
        }
    }
}

/// Recommendation prioritizer
pub struct RecommendationPrioritizer;

impl RecommendationPrioritizer {
    /// Prioritize recommendations based on context and constraints
    pub async fn prioritize_recommendations(
        recommendations: &[Recommendation],
        context_constraints: &ContextConstraints,
    ) -> Result<Vec<PrioritizedRecommendation>> {
        let mut prioritized = Vec::new();

        for (index, recommendation) in recommendations.iter().enumerate() {
            let adjusted_priority = Self::adjust_priority_for_constraints(recommendation, context_constraints);
            let feasibility_score = Self::calculate_feasibility(recommendation, context_constraints);
            let impact_score = Self::calculate_impact_score(recommendation);

            prioritized.push(PrioritizedRecommendation {
                recommendation: recommendation.clone(),
                adjusted_priority,
                feasibility_score,
                impact_score,
                execution_order: index + 1,
                dependencies: Self::identify_dependencies(recommendation, recommendations),
            });
        }

        // Sort by combined score
        prioritized.sort_by(|a, b| {
            let a_score = Self::calculate_combined_score(a);
            let b_score = Self::calculate_combined_score(b);
            b_score.partial_cmp(&a_score).unwrap()
        });

        // Update execution order
        for (i, rec) in prioritized.iter_mut().enumerate() {
            rec.execution_order = i + 1;
        }

        Ok(prioritized)
    }

    /// Adjust priority based on context constraints
    fn adjust_priority_for_constraints(
        recommendation: &Recommendation,
        constraints: &ContextConstraints,
    ) -> Priority {
        // If time-constrained, prioritize immediate actions
        if constraints.time_pressure && recommendation.urgency == Urgency::Immediate {
            return Priority::Critical;
        }

        // If resource-constrained, deprioritize resource-intensive recommendations
        if constraints.resource_limitations && 
           recommendation.implementation_timeline > Duration::from_secs(3600) {
            return match recommendation.priority {
                Priority::Critical => Priority::High,
                Priority::High => Priority::Medium,
                Priority::Medium => Priority::Low,
                Priority::Low => Priority::Low,
            };
        }

        recommendation.priority.clone()
    }

    /// Calculate feasibility score
    fn calculate_feasibility(recommendation: &Recommendation, constraints: &ContextConstraints) -> f64 {
        let mut score = 1.0;

        // Time feasibility
        if constraints.time_pressure && recommendation.implementation_timeline > Duration::from_secs(1800) {
            score *= 0.7;
        }

        // Resource feasibility
        if constraints.resource_limitations && recommendation.potential_risks.len() > 2 {
            score *= 0.8;
        }

        // Complexity feasibility
        if constraints.complexity_limitations && 
           recommendation.recommendation_type == RecommendationType::StrategyRefinement {
            score *= 0.6;
        }

        score
    }

    /// Calculate impact score
    fn calculate_impact_score(recommendation: &Recommendation) -> f64 {
        let benefit_score = recommendation.expected_benefits.len() as f64 * 0.2;
        let risk_penalty = recommendation.potential_risks.len() as f64 * 0.1;
        
        (benefit_score - risk_penalty).max(0.1)
    }

    /// Calculate combined score
    fn calculate_combined_score(prioritized: &PrioritizedRecommendation) -> f64 {
        let priority_weight = match prioritized.adjusted_priority {
            Priority::Critical => 1.0,
            Priority::High => 0.8,
            Priority::Medium => 0.6,
            Priority::Low => 0.4,
        };

        (priority_weight * 0.4) + (prioritized.feasibility_score * 0.3) + (prioritized.impact_score * 0.3)
    }

    /// Identify dependencies between recommendations
    fn identify_dependencies(
        recommendation: &Recommendation,
        all_recommendations: &[Recommendation],
    ) -> Vec<String> {
        let mut dependencies = Vec::new();

        // Simple dependency logic - immediate actions should come before others
        if recommendation.recommendation_type != RecommendationType::ImmediateAction {
            for other in all_recommendations {
                if other.recommendation_type == RecommendationType::ImmediateAction &&
                   other.recommendation_id != recommendation.recommendation_id {
                    dependencies.push(other.recommendation_id.clone());
                }
            }
        }

        dependencies
    }
}
