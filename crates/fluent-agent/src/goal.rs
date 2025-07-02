use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use crate::context::ExecutionContext;

/// Goal that the agent is working towards
/// 
/// Goals represent high-level objectives that the agent should achieve.
/// They provide direction and success criteria for agent execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub goal_id: String,
    pub description: String,
    pub goal_type: GoalType,
    pub priority: GoalPriority,
    pub success_criteria: Vec<String>,
    pub max_iterations: Option<u32>,
    pub timeout: Option<Duration>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Types of goals the agent can work on
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GoalType {
    CodeGeneration,
    CodeReview,
    Testing,
    Debugging,
    Refactoring,
    Documentation,
    Analysis,
    FileOperation,
    Communication,
    Planning,
    Learning,
    Research,
}

/// Priority levels for goals
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum GoalPriority {
    Low,
    Medium,
    High,
    Critical,
}

/// Result of goal execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalResult {
    pub success: bool,
    pub final_context: ExecutionContext,
    pub execution_summary: String,
    pub reasoning_steps: usize,
    pub actions_taken: usize,
    pub total_duration: Duration,
    pub final_output: Option<String>,
}

/// Builder for creating goals with fluent API
pub struct GoalBuilder {
    goal: Goal,
}

impl Goal {
    /// Create a new goal
    pub fn new(description: String, goal_type: GoalType) -> Self {
        Self {
            goal_id: uuid::Uuid::new_v4().to_string(),
            description,
            goal_type,
            priority: GoalPriority::Medium,
            success_criteria: Vec::new(),
            max_iterations: None,
            timeout: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a goal builder for fluent construction
    pub fn builder(description: String, goal_type: GoalType) -> GoalBuilder {
        GoalBuilder {
            goal: Self::new(description, goal_type),
        }
    }

    /// Check if the goal is achievable within constraints
    pub fn is_achievable(&self) -> bool {
        !self.description.is_empty() && !self.success_criteria.is_empty()
    }

    /// Get estimated complexity based on goal type and criteria
    pub fn get_complexity(&self) -> GoalComplexity {
        let criteria_count = self.success_criteria.len();
        
        match self.goal_type {
            GoalType::CodeGeneration | GoalType::Refactoring => {
                if criteria_count > 5 {
                    GoalComplexity::High
                } else if criteria_count > 2 {
                    GoalComplexity::Medium
                } else {
                    GoalComplexity::Low
                }
            }
            GoalType::Analysis | GoalType::Research => {
                if criteria_count > 3 {
                    GoalComplexity::Medium
                } else {
                    GoalComplexity::Low
                }
            }
            GoalType::FileOperation | GoalType::Communication => GoalComplexity::Low,
            _ => GoalComplexity::Medium,
        }
    }

    /// Get estimated duration based on complexity and type
    pub fn get_estimated_duration(&self) -> Duration {
        let base_duration = match self.goal_type {
            GoalType::CodeGeneration => Duration::from_secs(300), // 5 minutes
            GoalType::CodeReview => Duration::from_secs(180),     // 3 minutes
            GoalType::Testing => Duration::from_secs(240),       // 4 minutes
            GoalType::Debugging => Duration::from_secs(600),     // 10 minutes
            GoalType::Refactoring => Duration::from_secs(480),   // 8 minutes
            GoalType::Documentation => Duration::from_secs(360), // 6 minutes
            GoalType::Analysis => Duration::from_secs(420),      // 7 minutes
            GoalType::FileOperation => Duration::from_secs(60),  // 1 minute
            GoalType::Communication => Duration::from_secs(30),  // 30 seconds
            GoalType::Planning => Duration::from_secs(120),      // 2 minutes
            GoalType::Learning => Duration::from_secs(900),      // 15 minutes
            GoalType::Research => Duration::from_secs(1200),     // 20 minutes
        };

        // Adjust based on complexity
        match self.get_complexity() {
            GoalComplexity::Low => base_duration,
            GoalComplexity::Medium => base_duration * 2,
            GoalComplexity::High => base_duration * 4,
        }
    }

    /// Check if goal has timed out
    pub fn is_timed_out(&self, start_time: SystemTime) -> bool {
        if let Some(timeout) = self.timeout {
            SystemTime::now().duration_since(start_time).unwrap_or_default() > timeout
        } else {
            false
        }
    }

    /// Validate goal completeness and consistency
    pub fn validate(&self) -> Result<(), GoalValidationError> {
        if self.description.is_empty() {
            return Err(GoalValidationError::EmptyDescription);
        }

        if self.description.len() < 5 {
            return Err(GoalValidationError::DescriptionTooShort);
        }

        if self.success_criteria.is_empty() {
            return Err(GoalValidationError::NoSuccessCriteria);
        }

        if let Some(max_iter) = self.max_iterations {
            if max_iter == 0 {
                return Err(GoalValidationError::InvalidMaxIterations);
            }
        }

        if let Some(timeout) = self.timeout {
            if timeout.as_secs() == 0 {
                return Err(GoalValidationError::InvalidTimeout);
            }
        }

        Ok(())
    }

    /// Get goal summary for display
    pub fn get_summary(&self) -> String {
        format!(
            "Goal: {} ({}), Priority: {:?}, Criteria: {}",
            self.description,
            format!("{:?}", self.goal_type),
            self.priority,
            self.success_criteria.len()
        )
    }

    /// Add metadata to the goal
    pub fn add_metadata(&mut self, key: String, value: serde_json::Value) {
        self.metadata.insert(key, value);
    }

    /// Get metadata value
    pub fn get_metadata(&self, key: &str) -> Option<&serde_json::Value> {
        self.metadata.get(key)
    }
}

/// Complexity levels for goals
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GoalComplexity {
    Low,
    Medium,
    High,
}

/// Errors that can occur during goal validation
#[derive(Debug, Clone, PartialEq)]
pub enum GoalValidationError {
    EmptyDescription,
    DescriptionTooShort,
    NoSuccessCriteria,
    InvalidMaxIterations,
    InvalidTimeout,
}

impl std::fmt::Display for GoalValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GoalValidationError::EmptyDescription => write!(f, "Goal description cannot be empty"),
            GoalValidationError::DescriptionTooShort => write!(f, "Goal description is too short"),
            GoalValidationError::NoSuccessCriteria => write!(f, "Goal must have success criteria"),
            GoalValidationError::InvalidMaxIterations => write!(f, "Max iterations must be greater than 0"),
            GoalValidationError::InvalidTimeout => write!(f, "Timeout must be greater than 0"),
        }
    }
}

impl std::error::Error for GoalValidationError {}

impl GoalBuilder {
    /// Set goal priority
    pub fn priority(mut self, priority: GoalPriority) -> Self {
        self.goal.priority = priority;
        self
    }

    /// Add success criteria
    pub fn success_criteria(mut self, criteria: Vec<String>) -> Self {
        self.goal.success_criteria = criteria;
        self
    }

    /// Add a single success criterion
    pub fn success_criterion(mut self, criterion: String) -> Self {
        self.goal.success_criteria.push(criterion);
        self
    }

    /// Set maximum iterations
    pub fn max_iterations(mut self, max_iterations: u32) -> Self {
        self.goal.max_iterations = Some(max_iterations);
        self
    }

    /// Set timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.goal.timeout = Some(timeout);
        self
    }

    /// Add metadata
    pub fn metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.goal.metadata.insert(key, value);
        self
    }

    /// Build the goal
    pub fn build(self) -> Result<Goal, GoalValidationError> {
        self.goal.validate()?;
        Ok(self.goal)
    }

    /// Build the goal without validation (for testing)
    pub fn build_unchecked(self) -> Goal {
        self.goal
    }
}

/// Common goal templates for quick creation
pub struct GoalTemplates;

impl GoalTemplates {
    /// Create a code generation goal
    pub fn code_generation(description: String, language: String, requirements: Vec<String>) -> Goal {
        let mut criteria = vec![
            format!("Generate valid {} code", language),
            "Code compiles without errors".to_string(),
            "Code meets all requirements".to_string(),
        ];
        criteria.extend(requirements);

        Goal::builder(description, GoalType::CodeGeneration)
            .priority(GoalPriority::High)
            .success_criteria(criteria)
            .max_iterations(20)
            .timeout(Duration::from_secs(600))
            .metadata("language".to_string(), serde_json::json!(language))
            .build_unchecked()
    }

    /// Create a code review goal
    pub fn code_review(file_path: String, focus_areas: Vec<String>) -> Goal {
        Goal::builder(format!("Review code in {}", file_path), GoalType::CodeReview)
            .priority(GoalPriority::Medium)
            .success_criteria(vec![
                "Identify potential issues".to_string(),
                "Suggest improvements".to_string(),
                "Check code quality".to_string(),
            ])
            .success_criteria(focus_areas.iter().map(|area| format!("Review {}", area)).collect())
            .max_iterations(15)
            .timeout(Duration::from_secs(300))
            .metadata("file_path".to_string(), serde_json::json!(file_path))
            .build_unchecked()
    }

    /// Create a debugging goal
    pub fn debugging(issue_description: String, error_details: String) -> Goal {
        Goal::builder(format!("Debug issue: {}", issue_description), GoalType::Debugging)
            .priority(GoalPriority::High)
            .success_criteria(vec![
                "Identify root cause".to_string(),
                "Propose solution".to_string(),
                "Verify fix works".to_string(),
            ])
            .max_iterations(25)
            .timeout(Duration::from_secs(900))
            .metadata("issue_description".to_string(), serde_json::json!(issue_description))
            .metadata("error_details".to_string(), serde_json::json!(error_details))
            .build_unchecked()
    }

    /// Create a testing goal
    pub fn testing(component: String, test_types: Vec<String>) -> Goal {
        Goal::builder(format!("Create tests for {}", component), GoalType::Testing)
            .priority(GoalPriority::Medium)
            .success_criteria(vec![
                "Create comprehensive test suite".to_string(),
                "Achieve good test coverage".to_string(),
                "Tests pass successfully".to_string(),
            ])
            .success_criteria(test_types.iter().map(|t| format!("Include {} tests", t)).collect())
            .max_iterations(20)
            .timeout(Duration::from_secs(480))
            .metadata("component".to_string(), serde_json::json!(component))
            .build_unchecked()
    }

    /// Create a refactoring goal
    pub fn refactoring(target: String, objectives: Vec<String>) -> Goal {
        Goal::builder(format!("Refactor {}", target), GoalType::Refactoring)
            .priority(GoalPriority::Medium)
            .success_criteria(vec![
                "Improve code structure".to_string(),
                "Maintain functionality".to_string(),
                "Enhance readability".to_string(),
            ])
            .success_criteria(objectives)
            .max_iterations(30)
            .timeout(Duration::from_secs(720))
            .metadata("target".to_string(), serde_json::json!(target))
            .build_unchecked()
    }

    /// Create a documentation goal
    pub fn documentation(scope: String, doc_types: Vec<String>) -> Goal {
        Goal::builder(format!("Create documentation for {}", scope), GoalType::Documentation)
            .priority(GoalPriority::Low)
            .success_criteria(vec![
                "Create clear documentation".to_string(),
                "Include examples".to_string(),
                "Cover all features".to_string(),
            ])
            .success_criteria(doc_types.iter().map(|t| format!("Include {} documentation", t)).collect())
            .max_iterations(15)
            .timeout(Duration::from_secs(540))
            .metadata("scope".to_string(), serde_json::json!(scope))
            .build_unchecked()
    }

    /// Create an analysis goal
    pub fn analysis(subject: String, analysis_type: String) -> Goal {
        Goal::builder(format!("Analyze {} for {}", subject, analysis_type), GoalType::Analysis)
            .priority(GoalPriority::Medium)
            .success_criteria(vec![
                "Complete thorough analysis".to_string(),
                "Identify key insights".to_string(),
                "Provide recommendations".to_string(),
            ])
            .max_iterations(20)
            .timeout(Duration::from_secs(600))
            .metadata("subject".to_string(), serde_json::json!(subject))
            .metadata("analysis_type".to_string(), serde_json::json!(analysis_type))
            .build_unchecked()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_goal_creation() {
        let goal = Goal::new("Test goal".to_string(), GoalType::CodeGeneration);
        
        assert!(!goal.goal_id.is_empty());
        assert_eq!(goal.description, "Test goal");
        assert!(matches!(goal.goal_type, GoalType::CodeGeneration));
        assert_eq!(goal.priority, GoalPriority::Medium);
    }

    #[test]
    fn test_goal_builder() {
        let goal = Goal::builder("Test goal".to_string(), GoalType::Testing)
            .priority(GoalPriority::High)
            .success_criterion("Test passes".to_string())
            .max_iterations(10)
            .timeout(Duration::from_secs(300))
            .build()
            .unwrap();
        
        assert_eq!(goal.description, "Test goal");
        assert_eq!(goal.priority, GoalPriority::High);
        assert_eq!(goal.success_criteria.len(), 1);
        assert_eq!(goal.max_iterations, Some(10));
        assert_eq!(goal.timeout, Some(Duration::from_secs(300)));
    }

    #[test]
    fn test_goal_validation() {
        let invalid_goal = Goal {
            goal_id: "test".to_string(),
            description: "".to_string(),
            goal_type: GoalType::Analysis,
            priority: GoalPriority::Low,
            success_criteria: Vec::new(),
            max_iterations: None,
            timeout: None,
            metadata: HashMap::new(),
        };
        
        assert!(matches!(invalid_goal.validate(), Err(GoalValidationError::EmptyDescription)));
    }

    #[test]
    fn test_goal_complexity() {
        let simple_goal = Goal::builder("Simple task".to_string(), GoalType::FileOperation)
            .success_criterion("Complete task".to_string())
            .build()
            .unwrap();
        
        assert_eq!(simple_goal.get_complexity(), GoalComplexity::Low);
        
        let complex_goal = Goal::builder("Complex task".to_string(), GoalType::CodeGeneration)
            .success_criteria(vec![
                "Criterion 1".to_string(),
                "Criterion 2".to_string(),
                "Criterion 3".to_string(),
                "Criterion 4".to_string(),
                "Criterion 5".to_string(),
                "Criterion 6".to_string(),
            ])
            .build()
            .unwrap();
        
        assert_eq!(complex_goal.get_complexity(), GoalComplexity::High);
    }

    #[test]
    fn test_goal_templates() {
        let code_goal = GoalTemplates::code_generation(
            "Generate a function".to_string(),
            "Rust".to_string(),
            vec!["Must be async".to_string()],
        );
        
        assert!(matches!(code_goal.goal_type, GoalType::CodeGeneration));
        assert_eq!(code_goal.priority, GoalPriority::High);
        assert!(code_goal.success_criteria.len() >= 3);
        assert_eq!(code_goal.get_metadata("language"), Some(&serde_json::json!("Rust")));
    }

    #[test]
    fn test_goal_timeout() {
        let goal = Goal::builder("Test goal".to_string(), GoalType::Analysis)
            .timeout(Duration::from_secs(100))
            .success_criterion("Complete".to_string())
            .build()
            .unwrap();
        
        let start_time = SystemTime::now() - Duration::from_secs(150);
        assert!(goal.is_timed_out(start_time));
        
        let recent_start = SystemTime::now() - Duration::from_secs(50);
        assert!(!goal.is_timed_out(recent_start));
    }
}
