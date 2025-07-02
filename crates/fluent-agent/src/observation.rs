use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;

use crate::action::ActionResult;
use crate::context::ExecutionContext;
use crate::orchestrator::{Observation, ObservationType};

/// Trait for observation processors that can analyze action results and environment changes
#[async_trait]
pub trait ObservationProcessor: Send + Sync {
    /// Process an action result and generate observations
    async fn process(&self, action_result: ActionResult, context: &ExecutionContext) -> Result<Observation>;
    
    /// Process environment changes and generate observations
    async fn process_environment_change(&self, change: EnvironmentChange, context: &ExecutionContext) -> Result<Observation>;
    
    /// Get the processing capabilities of this processor
    fn get_capabilities(&self) -> Vec<ProcessingCapability>;
}

/// Capabilities that an observation processor can provide
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessingCapability {
    ActionResultAnalysis,
    EnvironmentMonitoring,
    PatternRecognition,
    AnomalyDetection,
    ImpactAssessment,
    LearningExtraction,
}

/// Environment change that can be observed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentChange {
    pub change_id: String,
    pub timestamp: SystemTime,
    pub change_type: EnvironmentChangeType,
    pub description: String,
    pub affected_components: Vec<String>,
    pub severity: ChangeSeverity,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Types of environment changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EnvironmentChangeType {
    FileSystemChange,
    ProcessStateChange,
    NetworkStateChange,
    ResourceAvailabilityChange,
    ConfigurationChange,
    ExternalServiceChange,
}

/// Severity of environment changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeSeverity {
    Informational,
    Minor,
    Moderate,
    Major,
    Critical,
}

/// Comprehensive observation processor that analyzes multiple aspects of execution
pub struct ComprehensiveObservationProcessor {
    result_analyzer: Box<dyn ResultAnalyzer>,
    pattern_detector: Box<dyn PatternDetector>,
    impact_assessor: Box<dyn ImpactAssessor>,
    learning_extractor: Box<dyn LearningExtractor>,
    capabilities: Vec<ProcessingCapability>,
}

/// Analyzes action results for insights
#[async_trait]
pub trait ResultAnalyzer: Send + Sync {
    async fn analyze(&self, result: &ActionResult, context: &ExecutionContext) -> Result<ResultAnalysis>;
}

/// Detects patterns in observations and execution history
#[async_trait]
pub trait PatternDetector: Send + Sync {
    async fn detect_patterns(&self, observations: &[Observation], context: &ExecutionContext) -> Result<Vec<Pattern>>;
}

/// Assesses the impact of actions and changes
#[async_trait]
pub trait ImpactAssessor: Send + Sync {
    async fn assess_impact(&self, result: &ActionResult, context: &ExecutionContext) -> Result<ImpactAssessment>;
}

/// Extracts learning insights from observations
#[async_trait]
pub trait LearningExtractor: Send + Sync {
    async fn extract_learning(&self, observation: &Observation, context: &ExecutionContext) -> Result<LearningInsight>;
}

/// Analysis of action results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultAnalysis {
    pub success_indicators: Vec<String>,
    pub failure_indicators: Vec<String>,
    pub performance_metrics: HashMap<String, f64>,
    pub quality_score: f64,
    pub unexpected_outcomes: Vec<String>,
    pub recommendations: Vec<String>,
}

/// Detected pattern in execution or observations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub pattern_id: String,
    pub pattern_type: PatternType,
    pub description: String,
    pub frequency: u32,
    pub confidence: f64,
    pub implications: Vec<String>,
}

/// Types of patterns that can be detected
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternType {
    SuccessPattern,
    FailurePattern,
    PerformancePattern,
    BehaviorPattern,
    ResourceUsagePattern,
    ErrorPattern,
}

/// Assessment of action impact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactAssessment {
    pub positive_impacts: Vec<Impact>,
    pub negative_impacts: Vec<Impact>,
    pub overall_impact_score: f64,
    pub risk_factors: Vec<String>,
    pub mitigation_suggestions: Vec<String>,
}

/// Individual impact item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Impact {
    pub description: String,
    pub magnitude: f64,
    pub affected_areas: Vec<String>,
    pub duration: ImpactDuration,
}

/// Duration of impact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImpactDuration {
    Immediate,
    ShortTerm,
    MediumTerm,
    LongTerm,
    Permanent,
}

/// Learning insight extracted from observations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningInsight {
    pub insight_type: InsightType,
    pub description: String,
    pub confidence: f64,
    pub applicability: Vec<String>,
    pub actionable_recommendations: Vec<String>,
}

/// Types of learning insights
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InsightType {
    StrategyImprovement,
    ToolOptimization,
    ProcessEnhancement,
    ErrorPrevention,
    PerformanceOptimization,
    QualityImprovement,
}

impl ComprehensiveObservationProcessor {
    /// Create a new comprehensive observation processor
    pub fn new(
        result_analyzer: Box<dyn ResultAnalyzer>,
        pattern_detector: Box<dyn PatternDetector>,
        impact_assessor: Box<dyn ImpactAssessor>,
        learning_extractor: Box<dyn LearningExtractor>,
    ) -> Self {
        Self {
            result_analyzer,
            pattern_detector,
            impact_assessor,
            learning_extractor,
            capabilities: vec![
                ProcessingCapability::ActionResultAnalysis,
                ProcessingCapability::EnvironmentMonitoring,
                ProcessingCapability::PatternRecognition,
                ProcessingCapability::AnomalyDetection,
                ProcessingCapability::ImpactAssessment,
                ProcessingCapability::LearningExtraction,
            ],
        }
    }

    /// Calculate relevance score for an observation
    fn calculate_relevance_score(&self, result: &ActionResult, context: &ExecutionContext) -> f64 {
        let mut score: f64 = 0.5; // Base score
        
        // Increase score for successful actions
        if result.success {
            score += 0.2;
        } else {
            score += 0.3; // Failures are often more relevant for learning
        }
        
        // Increase score for actions that produce output
        if result.output.is_some() {
            score += 0.1;
        }
        
        // Increase score for actions with side effects
        if !result.side_effects.is_empty() {
            score += 0.1;
        }
        
        // Increase score for actions related to current goal
        if self.is_goal_related(result, context) {
            score += 0.2;
        }
        
        score.min(1.0)
    }

    /// Check if action result is related to current goal
    fn is_goal_related(&self, result: &ActionResult, context: &ExecutionContext) -> bool {
        if let Some(goal) = context.get_current_goal() {
            let goal_keywords = self.extract_keywords(&goal.description);
            let result_text = format!("{:?}", result);
            
            goal_keywords.iter().any(|keyword| result_text.to_lowercase().contains(&keyword.to_lowercase()))
        } else {
            false
        }
    }

    /// Extract keywords from text
    fn extract_keywords(&self, text: &str) -> Vec<String> {
        text.split_whitespace()
            .filter(|word| word.len() > 3)
            .map(|word| word.to_lowercase())
            .collect()
    }

    /// Generate impact assessment description
    fn generate_impact_assessment(&self, result: &ActionResult) -> String {
        if result.success {
            if result.side_effects.is_empty() {
                "Action completed successfully with no side effects".to_string()
            } else {
                format!("Action completed successfully with {} side effects", result.side_effects.len())
            }
        } else {
            format!("Action failed: {}", result.error.as_deref().unwrap_or("Unknown error"))
        }
    }
}

#[async_trait]
impl ObservationProcessor for ComprehensiveObservationProcessor {
    async fn process(&self, action_result: ActionResult, context: &ExecutionContext) -> Result<Observation> {
        // Analyze the action result
        let analysis = self.result_analyzer.analyze(&action_result, context).await?;
        
        // Assess impact
        let impact = self.impact_assessor.assess_impact(&action_result, context).await?;
        
        // Calculate relevance score
        let relevance_score = self.calculate_relevance_score(&action_result, context);
        
        // Generate observation
        let observation = Observation {
            observation_id: uuid::Uuid::new_v4().to_string(),
            timestamp: SystemTime::now(),
            observation_type: ObservationType::ActionResult,
            content: format!(
                "Action {} ({}): {}. Analysis: Quality score {:.2}, {} success indicators, {} failure indicators. Impact: {:.2} overall score.",
                action_result.action_id,
                format!("{:?}", action_result.action_type),
                if action_result.success { "SUCCESS" } else { "FAILED" },
                analysis.quality_score,
                analysis.success_indicators.len(),
                analysis.failure_indicators.len(),
                impact.overall_impact_score
            ),
            source: "ComprehensiveObservationProcessor".to_string(),
            relevance_score,
            impact_assessment: Some(self.generate_impact_assessment(&action_result)),
        };
        
        // Extract learning insights
        let _learning = self.learning_extractor.extract_learning(&observation, context).await?;
        
        Ok(observation)
    }

    async fn process_environment_change(&self, change: EnvironmentChange, _context: &ExecutionContext) -> Result<Observation> {
        let relevance_score = match change.severity {
            ChangeSeverity::Critical => 1.0,
            ChangeSeverity::Major => 0.8,
            ChangeSeverity::Moderate => 0.6,
            ChangeSeverity::Minor => 0.4,
            ChangeSeverity::Informational => 0.2,
        };

        let observation = Observation {
            observation_id: uuid::Uuid::new_v4().to_string(),
            timestamp: change.timestamp,
            observation_type: ObservationType::EnvironmentChange,
            content: format!(
                "Environment change detected: {} ({}). Severity: {:?}. Affected components: {}",
                change.description,
                change.change_id,
                change.severity,
                change.affected_components.join(", ")
            ),
            source: "EnvironmentMonitor".to_string(),
            relevance_score,
            impact_assessment: Some(format!("Environment change with {:?} severity", change.severity)),
        };

        Ok(observation)
    }

    fn get_capabilities(&self) -> Vec<ProcessingCapability> {
        self.capabilities.clone()
    }
}

// Default implementations for testing and basic functionality

/// Basic result analyzer implementation
pub struct BasicResultAnalyzer;

#[async_trait]
impl ResultAnalyzer for BasicResultAnalyzer {
    async fn analyze(&self, result: &ActionResult, _context: &ExecutionContext) -> Result<ResultAnalysis> {
        let mut success_indicators = Vec::new();
        let mut failure_indicators = Vec::new();
        let mut performance_metrics = HashMap::new();
        
        if result.success {
            success_indicators.push("Action completed successfully".to_string());
            if let Some(output) = &result.output {
                success_indicators.push(format!("Generated output: {} characters", output.len()));
            }
        } else {
            failure_indicators.push("Action failed".to_string());
            if let Some(error) = &result.error {
                failure_indicators.push(format!("Error: {}", error));
            }
        }
        
        // Basic performance metrics
        performance_metrics.insert("execution_time_ms".to_string(), result.execution_time.as_millis() as f64);
        performance_metrics.insert("side_effects_count".to_string(), result.side_effects.len() as f64);
        
        let quality_score = if result.success {
            0.8 - (result.execution_time.as_secs() as f64 * 0.01) // Penalize long execution times
        } else {
            0.2
        };
        
        Ok(ResultAnalysis {
            success_indicators,
            failure_indicators,
            performance_metrics,
            quality_score: quality_score.max(0.0).min(1.0),
            unexpected_outcomes: Vec::new(),
            recommendations: vec!["Continue with current approach".to_string()],
        })
    }
}

/// Basic pattern detector implementation
pub struct BasicPatternDetector;

#[async_trait]
impl PatternDetector for BasicPatternDetector {
    async fn detect_patterns(&self, observations: &[Observation], _context: &ExecutionContext) -> Result<Vec<Pattern>> {
        let mut patterns = Vec::new();
        
        // Simple pattern: consecutive failures
        let failure_count = observations.iter()
            .rev()
            .take(5)
            .filter(|obs| obs.content.contains("FAILED"))
            .count();
        
        if failure_count >= 3 {
            patterns.push(Pattern {
                pattern_id: uuid::Uuid::new_v4().to_string(),
                pattern_type: PatternType::FailurePattern,
                description: "Multiple consecutive failures detected".to_string(),
                frequency: failure_count as u32,
                confidence: 0.8,
                implications: vec!["Strategy adjustment may be needed".to_string()],
            });
        }
        
        Ok(patterns)
    }
}

/// Basic impact assessor implementation
pub struct BasicImpactAssessor;

#[async_trait]
impl ImpactAssessor for BasicImpactAssessor {
    async fn assess_impact(&self, result: &ActionResult, _context: &ExecutionContext) -> Result<ImpactAssessment> {
        let mut positive_impacts = Vec::new();
        let mut negative_impacts = Vec::new();
        
        if result.success {
            positive_impacts.push(Impact {
                description: "Action completed successfully".to_string(),
                magnitude: 0.7,
                affected_areas: vec!["Goal progress".to_string()],
                duration: ImpactDuration::ShortTerm,
            });
        } else {
            negative_impacts.push(Impact {
                description: "Action failed to complete".to_string(),
                magnitude: 0.5,
                affected_areas: vec!["Goal progress".to_string()],
                duration: ImpactDuration::Immediate,
            });
        }
        
        let overall_impact_score = if result.success { 0.7 } else { -0.3 };
        
        Ok(ImpactAssessment {
            positive_impacts,
            negative_impacts,
            overall_impact_score,
            risk_factors: Vec::new(),
            mitigation_suggestions: Vec::new(),
        })
    }
}

/// Basic learning extractor implementation
pub struct BasicLearningExtractor;

#[async_trait]
impl LearningExtractor for BasicLearningExtractor {
    async fn extract_learning(&self, observation: &Observation, _context: &ExecutionContext) -> Result<LearningInsight> {
        let insight_type = if observation.content.contains("SUCCESS") {
            InsightType::StrategyImprovement
        } else if observation.content.contains("FAILED") {
            InsightType::ErrorPrevention
        } else {
            InsightType::ProcessEnhancement
        };
        
        Ok(LearningInsight {
            insight_type,
            description: format!("Learning from observation: {}", observation.observation_id),
            confidence: observation.relevance_score,
            applicability: vec!["Current goal".to_string()],
            actionable_recommendations: vec!["Continue monitoring".to_string()],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator::ActionType;
    use crate::orchestrator::ActionResult as OrchActionResult;
    use std::time::Duration;

    #[tokio::test]
    async fn test_basic_observation_processing() {
        let processor = ComprehensiveObservationProcessor::new(
            Box::new(BasicResultAnalyzer),
            Box::new(BasicPatternDetector),
            Box::new(BasicImpactAssessor),
            Box::new(BasicLearningExtractor),
        );

        let action_result = ActionResult {
            action_id: "test-action".to_string(),
            action_type: ActionType::ToolExecution,
            parameters: HashMap::new(),
            result: OrchActionResult {
                success: true,
                output: Some("Test output".to_string()),
                error: None,
                metadata: HashMap::new(),
            },
            execution_time: Duration::from_millis(100),
            success: true,
            output: Some("Test output".to_string()),
            error: None,
            metadata: HashMap::new(),
            side_effects: Vec::new(),
        };

        let context = ExecutionContext::default();
        let observation = processor.process(action_result, &context).await.unwrap();

        assert!(!observation.observation_id.is_empty());
        assert!(observation.content.contains("SUCCESS"));
        assert!(observation.relevance_score > 0.0);
    }

    #[test]
    fn test_environment_change_creation() {
        let change = EnvironmentChange {
            change_id: "test-change".to_string(),
            timestamp: SystemTime::now(),
            change_type: EnvironmentChangeType::FileSystemChange,
            description: "File modified".to_string(),
            affected_components: vec!["file_system".to_string()],
            severity: ChangeSeverity::Minor,
            metadata: HashMap::new(),
        };

        assert_eq!(change.change_id, "test-change");
        assert!(matches!(change.change_type, EnvironmentChangeType::FileSystemChange));
        assert!(matches!(change.severity, ChangeSeverity::Minor));
    }
}
