use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use crate::context::ExecutionContext;

/// Format duration for display
fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.as_secs();
    if total_seconds < 60 {
        format!("{}s", total_seconds)
    } else if total_seconds < 3600 {
        format!("{}m", total_seconds / 60)
    } else {
        format!("{}h {}m", total_seconds / 3600, (total_seconds % 3600) / 60)
    }
}

/// Sub-goal that contributes to the main goal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubGoal {
    pub id: String,
    pub description: String,
    pub goal_type: GoalType,
    pub required_tools: Vec<String>,
    pub estimated_duration: Duration,
    pub dependencies: Vec<String>, // IDs of other sub-goals this depends on
    pub success_criteria: Vec<String>,
    pub status: SubGoalStatus,
}

/// Status of a sub-goal
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SubGoalStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

/// Dependency relationship between goals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalDependency {
    pub from_goal: String,
    pub to_goal: String,
    pub dependency_type: DependencyType,
}

/// Types of dependencies between goals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DependencyType {
    MustCompleteBefore,  // from_goal must complete before to_goal starts
    SharesResources,     // goals share resources and may conflict
    RelatedOutput,       // to_goal uses output from from_goal
}

/// Execution plan for a goal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub milestones: Vec<Milestone>,
    pub required_tools: Vec<String>,
    pub estimated_total_duration: Duration,
    pub risk_assessment: RiskLevel,
    pub parallel_opportunities: Vec<String>,
}

/// Milestone in goal execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    pub id: String,
    pub description: String,
    pub estimated_duration: Duration,
    pub required_tools: Vec<String>,
    pub success_criteria: Vec<String>,
}

/// Risk assessment levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

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
    pub sub_goals: Vec<SubGoal>,
    pub dependencies: Vec<GoalDependency>,
    pub execution_plan: Option<ExecutionPlan>,
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

impl std::fmt::Display for GoalType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            GoalType::CodeGeneration => "code_generation",
            GoalType::CodeReview => "code_review",
            GoalType::Testing => "testing",
            GoalType::Debugging => "debugging",
            GoalType::Refactoring => "refactoring",
            GoalType::Documentation => "documentation",
            GoalType::Analysis => "analysis",
            GoalType::FileOperation => "file_operation",
            GoalType::Communication => "communication",
            GoalType::Planning => "planning",
            GoalType::Learning => "learning",
            GoalType::Research => "research",
        };
        write!(f, "{}", s)
    }
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
        let mut goal = Self {
            goal_id: uuid::Uuid::new_v4().to_string(),
            description,
            goal_type,
            priority: GoalPriority::Medium,
            success_criteria: Vec::new(),
            max_iterations: None,
            timeout: None,
            metadata: HashMap::new(),
            sub_goals: Vec::new(),
            dependencies: Vec::new(),
            execution_plan: None,
        };

        // Try to analyze and decompose immediately, but don't fail if it doesn't work
        let _ = goal.analyze_and_decompose();

        goal
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
            GoalType::Testing => Duration::from_secs(240),        // 4 minutes
            GoalType::Debugging => Duration::from_secs(600),      // 10 minutes
            GoalType::Refactoring => Duration::from_secs(480),    // 8 minutes
            GoalType::Documentation => Duration::from_secs(360),  // 6 minutes
            GoalType::Analysis => Duration::from_secs(420),       // 7 minutes
            GoalType::FileOperation => Duration::from_secs(60),   // 1 minute
            GoalType::Communication => Duration::from_secs(30),   // 30 seconds
            GoalType::Planning => Duration::from_secs(120),       // 2 minutes
            GoalType::Learning => Duration::from_secs(900),       // 15 minutes
            GoalType::Research => Duration::from_secs(1200),      // 20 minutes
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
            SystemTime::now()
                .duration_since(start_time)
                .unwrap_or_default()
                > timeout
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

    /// Analyze goal and create decomposition plan
    pub fn analyze_and_decompose(&mut self) -> Result<(), GoalAnalysisError> {
        // Analyze the goal description for complexity patterns
        let analysis = self.analyze_goal_complexity()?;

        // Decompose into sub-goals if complex enough
        if analysis.needs_decomposition {
            self.decompose_into_sub_goals(&analysis)?;
        }

        // Create execution plan
        self.create_execution_plan(&analysis)?;

        Ok(())
    }

    /// Analyze goal complexity and decomposition needs
    fn analyze_goal_complexity(&self) -> Result<GoalAnalysis, GoalAnalysisError> {
        let description_words = self.description.split_whitespace().count();
        let has_multiple_tasks = self.description.contains(" and ") ||
                                self.description.contains(" then ") ||
                                self.description.contains(" also ");
        let has_file_operations = self.description.to_lowercase().contains("file") ||
                                self.description.to_lowercase().contains("create") ||
                                self.description.to_lowercase().contains("write");
        let has_analysis = self.description.to_lowercase().contains("analyze") ||
                          self.description.to_lowercase().contains("review") ||
                          self.description.to_lowercase().contains("check");
        let has_multiple_criteria = self.success_criteria.len() > 3;

        let complexity_score = if description_words > 50 { 3 }
                              else if description_words > 25 { 2 }
                              else { 1 } +
                              if has_multiple_tasks { 2 } else { 0 } +
                              if has_file_operations { 1 } else { 0 } +
                              if has_analysis { 1 } else { 0 } +
                              if has_multiple_criteria { 1 } else { 0 };

        let needs_decomposition = complexity_score >= 4;
        let estimated_sub_goals = if needs_decomposition {
            (complexity_score / 2).max(2).min(5)
        } else { 1 };

        Ok(GoalAnalysis {
            complexity_score,
            needs_decomposition,
            estimated_sub_goals,
            identified_tasks: self.extract_potential_tasks(),
            required_tools: self.identify_required_tools(),
        })
    }

    /// Extract potential tasks from goal description
    fn extract_potential_tasks(&self) -> Vec<String> {
        let mut tasks = Vec::new();
        let description = self.description.to_lowercase();

        // Look for task indicators
        if description.contains("create") && description.contains("test") {
            tasks.push("Create unit tests".to_string());
        }
        if description.contains("analyze") || description.contains("review") {
            tasks.push("Analyze code".to_string());
        }
        if description.contains("debug") || description.contains("fix") {
            tasks.push("Debug issues".to_string());
        }
        if description.contains("document") {
            tasks.push("Create documentation".to_string());
        }
        if description.contains("refactor") {
            tasks.push("Refactor code".to_string());
        }

        // Add success criteria as tasks if they look like actions
        for criterion in &self.success_criteria {
            if criterion.contains("create") || criterion.contains("implement") ||
               criterion.contains("add") || criterion.contains("write") {
                tasks.push(criterion.clone());
            }
        }

        tasks.truncate(5); // Limit to 5 tasks
        tasks
    }

    /// Identify tools that might be needed for this goal
    fn identify_required_tools(&self) -> Vec<String> {
        let mut tools = Vec::new();
        let description = self.description.to_lowercase();

        if description.contains("file") || description.contains("read") || description.contains("write") {
            tools.push("file_system".to_string());
        }
        if description.contains("http") || description.contains("api") || description.contains("web") {
            tools.push("http_client".to_string());
        }
        if description.contains("shell") || description.contains("command") || description.contains("run") {
            tools.push("shell".to_string());
        }
        if description.contains("search") || description.contains("find") {
            tools.push("grep".to_string());
        }
        if description.contains("test") {
            tools.push("test_runner".to_string());
        }

        tools
    }

    /// Decompose goal into sub-goals
    fn decompose_into_sub_goals(&mut self, analysis: &GoalAnalysis) -> Result<(), GoalAnalysisError> {
        let mut sub_goals = Vec::new();

        // Create sub-goals from identified tasks
        for (i, task) in analysis.identified_tasks.iter().enumerate() {
            let sub_goal = SubGoal {
                id: format!("{}_sub_{}", self.goal_id, i),
                description: task.clone(),
                goal_type: self.infer_sub_goal_type(task),
                required_tools: self.identify_tools_for_task(task),
                estimated_duration: self.estimate_task_duration(task),
                dependencies: Vec::new(), // Will be set after all sub-goals are created
                success_criteria: vec![format!("Complete: {}", task)],
                status: SubGoalStatus::Pending,
            };
            sub_goals.push(sub_goal);
        }

        // Set up dependencies between sub-goals
        self.setup_sub_goal_dependencies(&mut sub_goals);

        self.sub_goals = sub_goals;
        Ok(())
    }

    /// Infer goal type for a sub-task
    fn infer_sub_goal_type(&self, task: &str) -> GoalType {
        let task_lower = task.to_lowercase();
        if task_lower.contains("test") || task_lower.contains("spec") {
            GoalType::Testing
        } else if task_lower.contains("debug") || task_lower.contains("fix") || task_lower.contains("error") {
            GoalType::Debugging
        } else if task_lower.contains("analyze") || task_lower.contains("review") || task_lower.contains("check") {
            GoalType::Analysis
        } else if task_lower.contains("document") {
            GoalType::Documentation
        } else if task_lower.contains("refactor") {
            GoalType::Refactoring
        } else if task_lower.contains("file") || task_lower.contains("create") || task_lower.contains("write") {
            GoalType::FileOperation
        } else {
            GoalType::CodeGeneration
        }
    }

    /// Identify tools needed for a specific task
    fn identify_tools_for_task(&self, task: &str) -> Vec<String> {
        let mut tools = Vec::new();
        let task_lower = task.to_lowercase();

        if task_lower.contains("file") || task_lower.contains("read") || task_lower.contains("write") {
            tools.push("file_system".to_string());
        }
        if task_lower.contains("http") || task_lower.contains("api") {
            tools.push("http_client".to_string());
        }
        if task_lower.contains("shell") || task_lower.contains("command") {
            tools.push("shell".to_string());
        }
        if task_lower.contains("search") {
            tools.push("grep".to_string());
        }
        if task_lower.contains("test") {
            tools.push("test_runner".to_string());
        }

        tools
    }

    /// Estimate duration for a task
    fn estimate_task_duration(&self, task: &str) -> Duration {
        let task_lower = task.to_lowercase();
        if task_lower.contains("analyze") || task_lower.contains("review") {
            Duration::from_secs(180) // 3 minutes
        } else if task_lower.contains("debug") || task_lower.contains("fix") {
            Duration::from_secs(300) // 5 minutes
        } else if task_lower.contains("test") {
            Duration::from_secs(240) // 4 minutes
        } else if task_lower.contains("document") {
            Duration::from_secs(360) // 6 minutes
        } else {
            Duration::from_secs(120) // 2 minutes default
        }
    }

    /// Set up dependencies between sub-goals
    fn setup_sub_goal_dependencies(&mut self, sub_goals: &mut [SubGoal]) {
        // Simple dependency logic: analysis/review tasks should come before implementation
        let mut analysis_indices = Vec::new();
        let mut implementation_indices = Vec::new();

        for (i, sub_goal) in sub_goals.iter().enumerate() {
            match sub_goal.goal_type {
                GoalType::Analysis | GoalType::CodeReview => analysis_indices.push(i),
                GoalType::CodeGeneration | GoalType::FileOperation | GoalType::Testing => implementation_indices.push(i),
                _ => {}
            }
        }

        // Make implementation tasks depend on analysis tasks
        for &impl_idx in &implementation_indices {
            for &analysis_idx in &analysis_indices {
                if analysis_idx != impl_idx {
                    sub_goals[impl_idx].dependencies.push(sub_goals[analysis_idx].id.clone());
                }
            }
        }
    }

    /// Create execution plan for the goal
    fn create_execution_plan(&mut self, analysis: &GoalAnalysis) -> Result<(), GoalAnalysisError> {
        let mut milestones = Vec::new();
        let mut total_duration = Duration::from_secs(0);
        let mut all_tools = analysis.required_tools.clone();

        if self.sub_goals.is_empty() {
            // Single milestone for simple goals
            milestones.push(Milestone {
                id: format!("{}_milestone_1", self.goal_id),
                description: format!("Complete goal: {}", self.description),
                estimated_duration: self.get_estimated_duration(),
                required_tools: analysis.required_tools.clone(),
                success_criteria: self.success_criteria.clone(),
            });
            total_duration = self.get_estimated_duration();
        } else {
            // Multiple milestones for complex goals
            for (i, sub_goal) in self.sub_goals.iter().enumerate() {
                milestones.push(Milestone {
                    id: format!("{}_milestone_{}", self.goal_id, i + 1),
                    description: sub_goal.description.clone(),
                    estimated_duration: sub_goal.estimated_duration,
                    required_tools: sub_goal.required_tools.clone(),
                    success_criteria: sub_goal.success_criteria.clone(),
                });
                total_duration += sub_goal.estimated_duration;
                all_tools.extend(sub_goal.required_tools.clone());
            }
        }

        // Remove duplicates from tools
        all_tools.sort();
        all_tools.dedup();

        let risk_level = if analysis.complexity_score >= 6 {
            RiskLevel::High
        } else if analysis.complexity_score >= 4 {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        };

        // Identify parallel opportunities
        let parallel_opportunities = self.identify_parallel_opportunities();

        self.execution_plan = Some(ExecutionPlan {
            milestones,
            required_tools: all_tools,
            estimated_total_duration: total_duration,
            risk_assessment: risk_level,
            parallel_opportunities,
        });

        Ok(())
    }

    /// Identify opportunities for parallel execution
    fn identify_parallel_opportunities(&self) -> Vec<String> {
        let mut opportunities = Vec::new();

        if self.sub_goals.len() >= 2 {
            // Check if any sub-goals have no dependencies
            let independent_goals: Vec<_> = self.sub_goals.iter()
                .filter(|sg| sg.dependencies.is_empty())
                .collect();

            if independent_goals.len() >= 2 {
                opportunities.push("Multiple independent sub-tasks can run in parallel".to_string());
            }
        }

        opportunities
    }

    /// Get execution plan summary for display
    pub fn get_execution_plan_summary(&self) -> String {
        if let Some(plan) = &self.execution_plan {
            format!(
                "Execution Plan: {} milestones, {} total duration, {} risk, {} tools required",
                plan.milestones.len(),
                format_duration(plan.estimated_total_duration),
                format!("{:?}", plan.risk_assessment).to_lowercase(),
                plan.required_tools.len()
            )
        } else {
            "No execution plan available".to_string()
        }
    }
}

/// Analysis result for goal decomposition
struct GoalAnalysis {
    complexity_score: u32,
    needs_decomposition: bool,
    estimated_sub_goals: u32,
    identified_tasks: Vec<String>,
    required_tools: Vec<String>,
}

/// Errors that can occur during goal analysis
#[derive(Debug, Clone, PartialEq)]
pub enum GoalAnalysisError {
    TooComplex,
    InvalidStructure,
    MissingInformation,
}

impl std::fmt::Display for GoalAnalysisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GoalAnalysisError::TooComplex => write!(f, "Goal is too complex to analyze"),
            GoalAnalysisError::InvalidStructure => write!(f, "Goal has invalid structure"),
            GoalAnalysisError::MissingInformation => write!(f, "Goal is missing required information"),
        }
    }
}

impl std::error::Error for GoalAnalysisError {}

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
            GoalValidationError::InvalidMaxIterations => {
                write!(f, "Max iterations must be greater than 0")
            }
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
    pub fn code_generation(
        description: String,
        language: String,
        requirements: Vec<String>,
    ) -> Goal {
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
        Goal::builder(
            format!("Review code in {}", file_path),
            GoalType::CodeReview,
        )
        .priority(GoalPriority::Medium)
        .success_criteria(vec![
            "Identify potential issues".to_string(),
            "Suggest improvements".to_string(),
            "Check code quality".to_string(),
        ])
        .success_criteria(
            focus_areas
                .iter()
                .map(|area| format!("Review {}", area))
                .collect(),
        )
        .max_iterations(15)
        .timeout(Duration::from_secs(300))
        .metadata("file_path".to_string(), serde_json::json!(file_path))
        .build_unchecked()
    }

    /// Create a debugging goal
    pub fn debugging(issue_description: String, error_details: String) -> Goal {
        Goal::builder(
            format!("Debug issue: {}", issue_description),
            GoalType::Debugging,
        )
        .priority(GoalPriority::High)
        .success_criteria(vec![
            "Identify root cause".to_string(),
            "Propose solution".to_string(),
            "Verify fix works".to_string(),
        ])
        .max_iterations(25)
        .timeout(Duration::from_secs(900))
        .metadata(
            "issue_description".to_string(),
            serde_json::json!(issue_description),
        )
        .metadata(
            "error_details".to_string(),
            serde_json::json!(error_details),
        )
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
            .success_criteria(
                test_types
                    .iter()
                    .map(|t| format!("Include {} tests", t))
                    .collect(),
            )
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
        Goal::builder(
            format!("Create documentation for {}", scope),
            GoalType::Documentation,
        )
        .priority(GoalPriority::Low)
        .success_criteria(vec![
            "Create clear documentation".to_string(),
            "Include examples".to_string(),
            "Cover all features".to_string(),
        ])
        .success_criteria(
            doc_types
                .iter()
                .map(|t| format!("Include {} documentation", t))
                .collect(),
        )
        .max_iterations(15)
        .timeout(Duration::from_secs(540))
        .metadata("scope".to_string(), serde_json::json!(scope))
        .build_unchecked()
    }

    /// Create an analysis goal
    pub fn analysis(subject: String, analysis_type: String) -> Goal {
        Goal::builder(
            format!("Analyze {} for {}", subject, analysis_type),
            GoalType::Analysis,
        )
        .priority(GoalPriority::Medium)
        .success_criteria(vec![
            "Complete thorough analysis".to_string(),
            "Identify key insights".to_string(),
            "Provide recommendations".to_string(),
        ])
        .max_iterations(20)
        .timeout(Duration::from_secs(600))
        .metadata("subject".to_string(), serde_json::json!(subject))
        .metadata(
            "analysis_type".to_string(),
            serde_json::json!(analysis_type),
        )
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
            sub_goals: Vec::new(),
            dependencies: Vec::new(),
            execution_plan: None,
        };

        assert!(matches!(
            invalid_goal.validate(),
            Err(GoalValidationError::EmptyDescription)
        ));
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
        assert_eq!(
            code_goal.get_metadata("language"),
            Some(&serde_json::json!("Rust"))
        );
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

    #[test]
    fn test_goal_analysis_and_decomposition() {
        let mut goal = Goal::builder(
            "Create a Rust function to calculate fibonacci numbers and write comprehensive unit tests for it".to_string(),
            GoalType::CodeGeneration,
        )
        .success_criteria(vec![
            "Function compiles without errors".to_string(),
            "Function returns correct fibonacci values".to_string(),
            "Unit tests pass".to_string(),
            "Code is well-documented".to_string(),
        ])
        .build()
        .unwrap();

        // Test analysis and decomposition
        goal.analyze_and_decompose().unwrap();

        // Should have decomposed into sub-goals
        assert!(!goal.sub_goals.is_empty());

        // Should have an execution plan
        assert!(goal.execution_plan.is_some());

        let plan = goal.execution_plan.as_ref().unwrap();
        assert!(!plan.milestones.is_empty());
        assert!(plan.estimated_total_duration > Duration::from_secs(0));
    }

    #[test]
    fn test_simple_goal_no_decomposition() {
        let mut goal = Goal::builder(
            "Say hello".to_string(),
            GoalType::Communication,
        )
        .success_criterion("Output hello message".to_string())
        .build()
        .unwrap();

        // Test analysis - should not decompose
        goal.analyze_and_decompose().unwrap();

        // Should not have decomposed
        assert!(goal.sub_goals.is_empty());

        // Should still have an execution plan
        assert!(goal.execution_plan.is_some());
    }

    #[test]
    fn test_execution_plan_summary() {
        let mut goal = Goal::builder("Test goal".to_string(), GoalType::CodeGeneration)
            .success_criterion("Complete task".to_string())
            .build()
            .unwrap();

        goal.analyze_and_decompose().unwrap();

        let summary = goal.get_execution_plan_summary();
        assert!(summary.contains("Execution Plan"));
        assert!(summary.contains("milestones"));
    }
}
