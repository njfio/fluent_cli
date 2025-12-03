use anyhow::{anyhow, Result};
use async_trait::async_trait;
use tracing::info;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use crate::context::ExecutionContext;
use crate::orchestrator::{ActionType, ReasoningResult};

/// Trait for action planners that can determine the best next action
#[async_trait]
pub trait ActionPlanner: Send + Sync {
    /// Plan the next action based on reasoning results and context
    async fn plan_action(
        &self,
        reasoning: ReasoningResult,
        context: &ExecutionContext,
    ) -> Result<ActionPlan>;

    /// Get the planning capabilities of this planner
    fn get_capabilities(&self) -> Vec<PlanningCapability>;

    /// Validate if this planner can handle the given action type
    fn can_plan(&self, action_type: &ActionType) -> bool;
}

/// Trait for action executors that can perform planned actions
#[async_trait]
pub trait ActionExecutor: Send + Sync {
    /// Execute a planned action and return the result
    async fn execute(
        &self,
        plan: ActionPlan,
        context: &mut ExecutionContext,
    ) -> Result<ActionResult>;

    /// Get the execution capabilities of this executor
    fn get_capabilities(&self) -> Vec<ExecutionCapability>;

    /// Validate if this executor can handle the given action type
    fn can_execute(&self, action_type: &ActionType) -> bool;
}

/// Capabilities that an action planner can provide
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlanningCapability {
    ToolSelection,
    ParameterOptimization,
    RiskAssessment,
    AlternativePlanning,
    ResourceAllocation,
    TimingOptimization,
}

/// Capabilities that an action executor can provide
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionCapability {
    ToolExecution,
    CodeGeneration,
    FileOperations,
    NetworkRequests,
    ProcessManagement,
    DataAnalysis,
    ErrorRecovery,
}

/// Planned action with all necessary details for execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionPlan {
    pub action_id: String,
    pub action_type: ActionType,
    pub description: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub expected_outcome: String,
    pub success_criteria: Vec<String>,
    pub confidence_score: f64,
    pub estimated_duration: Option<Duration>,
    pub risk_level: RiskLevel,
    pub alternatives: Vec<AlternativeAction>,
    pub prerequisites: Vec<String>,
}

/// Alternative action if the primary action fails
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlternativeAction {
    pub action_type: ActionType,
    pub description: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub trigger_conditions: Vec<String>,
}

/// Risk level assessment for an action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Result of action execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub action_id: String,
    pub action_type: ActionType,
    pub parameters: HashMap<String, serde_json::Value>,
    pub result: serde_json::Value,
    pub execution_time: Duration,
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub side_effects: Vec<SideEffect>,
}

/// Side effects produced by action execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SideEffect {
    pub effect_type: SideEffectType,
    pub description: String,
    pub impact_level: ImpactLevel,
}

/// Types of side effects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SideEffectType {
    FileModification,
    StateChange,
    NetworkActivity,
    ResourceConsumption,
    EnvironmentChange,
}

/// Impact level of side effects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImpactLevel {
    Minimal,
    Moderate,
    Significant,
    Major,
}

/// Intelligent action planner that uses reasoning to plan optimal actions
pub struct IntelligentActionPlanner {
    planning_strategies: HashMap<ActionType, Box<dyn PlanningStrategy>>,
    risk_assessor: Box<dyn RiskAssessor>,
    capabilities: Vec<PlanningCapability>,
}

/// Strategy for planning specific types of actions
#[async_trait]
pub trait PlanningStrategy: Send + Sync {
    async fn plan(
        &self,
        reasoning: &ReasoningResult,
        context: &ExecutionContext,
    ) -> Result<ActionPlan>;
}

/// Risk assessment for planned actions
#[async_trait]
pub trait RiskAssessor: Send + Sync {
    async fn assess_risk(&self, plan: &ActionPlan, context: &ExecutionContext)
        -> Result<RiskLevel>;
}

/// Comprehensive action executor that can handle multiple action types
pub struct ComprehensiveActionExecutor {
    tool_executor: Box<dyn ToolExecutor>,
    code_generator: Box<dyn CodeGenerator>,
    file_manager: Box<dyn FileManager>,
    capabilities: Vec<ExecutionCapability>,
}

/// Tool execution interface
#[async_trait]
pub trait ToolExecutor: Send + Sync {
    async fn execute_tool(
        &self,
        tool_name: &str,
        parameters: &HashMap<String, serde_json::Value>,
    ) -> Result<String>;
    fn get_available_tools(&self) -> Vec<String>;
}

/// Code generation interface
#[async_trait]
pub trait CodeGenerator: Send + Sync {
    async fn generate_code(
        &self,
        specification: &str,
        context: &ExecutionContext,
    ) -> Result<String>;
    fn get_supported_languages(&self) -> Vec<String>;
}

/// File management interface
#[async_trait]
pub trait FileManager: Send + Sync {
    async fn read_file(&self, path: &str) -> Result<String>;
    async fn write_file(&self, path: &str, content: &str) -> Result<()>;
    async fn create_directory(&self, path: &str) -> Result<()>;
    async fn delete_file(&self, path: &str) -> Result<()>;
}

impl IntelligentActionPlanner {
    /// Create a new intelligent action planner
    pub fn new(risk_assessor: Box<dyn RiskAssessor>) -> Self {
        let mut planner = Self {
            planning_strategies: HashMap::new(),
            risk_assessor,
            capabilities: vec![
                PlanningCapability::ToolSelection,
                PlanningCapability::ParameterOptimization,
                PlanningCapability::RiskAssessment,
                PlanningCapability::AlternativePlanning,
                PlanningCapability::ResourceAllocation,
                PlanningCapability::TimingOptimization,
            ],
        };

        // Register default planning strategies
        planner.register_strategy(ActionType::ToolExecution, Box::new(ToolPlanningStrategy));
        planner.register_strategy(ActionType::CodeGeneration, Box::new(CodePlanningStrategy));
        planner.register_strategy(ActionType::FileOperation, Box::new(FilePlanningStrategy));
        planner.register_strategy(ActionType::Analysis, Box::new(AnalysisPlanningStrategy));

        planner
    }

    /// Register a planning strategy for a specific action type
    pub fn register_strategy(
        &mut self,
        action_type: ActionType,
        strategy: Box<dyn PlanningStrategy>,
    ) {
        self.planning_strategies.insert(action_type, strategy);
    }

    /// Determine the best action type based on reasoning results
    fn determine_action_type(&self, reasoning: &ReasoningResult) -> ActionType {
        // Analyze reasoning output to determine appropriate action type
        let output = reasoning.reasoning_output.to_lowercase();

        // Priority 1: Explicit shell/command execution - use tools, not code
        if output.contains("shell")
            || output.contains("command")
            || output.contains("cargo test")
            || output.contains("cargo build")
            || output.contains("cargo run")
            || output.contains("run the test")
            || output.contains("execute the")
            || output.contains("list file")
            || output.contains("list dir")
        {
            return ActionType::ToolExecution;
        }

        // Priority 2: File operations (before generic "write" check)
        if (output.contains("read") && output.contains("file"))
            || (output.contains("write") && output.contains("file"))
            || output.contains("file operation")
            || output.contains("save to file")
            || output.contains("load from file")
        {
            return ActionType::FileOperation;
        }

        // Priority 3: Generic tool/execute keywords
        if output.contains("tool") || output.contains("execute") || output.contains("run") {
            return ActionType::ToolExecution;
        }

        // Priority 4: Code generation only for explicit creation tasks
        if output.contains("generate code")
            || output.contains("implement")
            || output.contains("create a program")
            || output.contains("write code")
            || (output.contains("code") && output.contains("new"))
        {
            return ActionType::CodeGeneration;
        }

        // Priority 5: Analysis
        if output.contains("analyze")
            || output.contains("examine")
            || output.contains("review")
        {
            return ActionType::Analysis;
        }

        // Priority 6: Communication
        if output.contains("communicate")
            || output.contains("message")
            || output.contains("notify")
        {
            return ActionType::Communication;
        }

        // Default: For unclear cases, try tool execution first as it's safer
        // than generating unnecessary code
        ActionType::ToolExecution
    }
}

#[async_trait]
impl ActionPlanner for IntelligentActionPlanner {
    async fn plan_action(
        &self,
        reasoning: ReasoningResult,
        context: &ExecutionContext,
    ) -> Result<ActionPlan> {
        // Determine the appropriate action type
        let mut action_type = self.determine_action_type(&reasoning);

        // 1) Adapt to repeated failures in recent observations (fallback to Analysis/recovery)
        let recent_failures = context
            .observations
            .iter()
            .rev()
            .take(5)
            .filter(|o| o.content.to_uppercase().contains("FAILED"))
            .count();
        if recent_failures >= 3 {
            action_type = crate::orchestrator::ActionType::Analysis;
        }

        // 2) Use reflection-based strategy adjustments to influence planning
        // Heuristic parsing of adjustments recorded in context (strings)
        let mut enforce_validation = false;
        let mut prefer_low_risk = false;
        let mut avoid_file_write = false;
        for adj in &context.strategy_adjustments {
            let text = adj.adjustments.join("; ").to_lowercase();
            if text.contains("quality") || text.contains("validation") {
                enforce_validation = true;
            }
            if text.contains("risk") || text.contains("mitigation") {
                prefer_low_risk = true;
            }
            if text.contains("avoid write") || text.contains("defer write") {
                avoid_file_write = true;
            }
        }

        // Get the planning strategy for this action type
        let strategy = self.planning_strategies.get(&action_type).ok_or_else(|| {
            anyhow!(
                "No planning strategy available for action type: {:?}",
                action_type
            )
        })?;

        // Plan the action using the strategy
        let mut plan = strategy.plan(&reasoning, context).await?;

        // Assess risk
        plan.risk_level = self.risk_assessor.assess_risk(&plan, context).await?;
        plan.confidence_score = reasoning.confidence_score;

        // Apply reflection-derived preferences
        if prefer_low_risk {
            // If current plan is high risk, switch to analysis before executing
            if matches!(plan.risk_level, RiskLevel::High | RiskLevel::Critical) {
                plan.action_type = crate::orchestrator::ActionType::Analysis;
                plan.description = "Perform diagnostic analysis due to high risk".to_string();
                plan.parameters.insert(
                    "analysis_type".to_string(),
                    serde_json::json!("risk_assessment"),
                );
                plan.expected_outcome = "Diagnostics collected".to_string();
                plan.risk_level = RiskLevel::Low;
            }
        }

        if enforce_validation {
            // Insert a validation analysis step if we just generated code or plan is file write
            if matches!(action_type, crate::orchestrator::ActionType::FileOperation)
                || matches!(action_type, crate::orchestrator::ActionType::CodeGeneration)
            {
                plan.action_type = crate::orchestrator::ActionType::Analysis;
                plan.description = "Validate generated artifact before persisting".to_string();
                plan.parameters
                    .insert("analysis_type".to_string(), serde_json::json!("validation"));
                plan.expected_outcome = "Validation report produced".to_string();
                plan.risk_level = RiskLevel::Low;
            }
        }

        if avoid_file_write
            && matches!(
                plan.action_type,
                crate::orchestrator::ActionType::FileOperation
            )
        {
            // Defer file writes; do planning/analysis instead
            plan.action_type = crate::orchestrator::ActionType::Planning;
            plan.description = "Plan safe persistence strategy (write deferred)".to_string();
            plan.parameters.insert(
                "scope".to_string(),
                serde_json::json!("persistence_strategy"),
            );
            plan.expected_outcome = "Persistence plan drafted".to_string();
            plan.risk_level = RiskLevel::Low;
        }

        // Generate alternatives if risk is high
        if matches!(plan.risk_level, RiskLevel::High | RiskLevel::Critical) {
            plan.alternatives = self.generate_alternatives(&plan, context)?;
        }

        // On failure streak, add recovery alternatives explicitly
        if recent_failures >= 3 {
            plan.alternatives.push(AlternativeAction {
                action_type: crate::orchestrator::ActionType::Analysis,
                description: "Run error diagnostics and refine plan".to_string(),
                parameters: {
                    let mut m = HashMap::new();
                    m.insert(
                        "analysis_type".to_string(),
                        serde_json::json!("error_recovery"),
                    );
                    m
                },
                trigger_conditions: vec!["consecutive_failures>=3".to_string()],
            });
        }

        Ok(plan)
    }

    fn get_capabilities(&self) -> Vec<PlanningCapability> {
        self.capabilities.clone()
    }

    fn can_plan(&self, action_type: &ActionType) -> bool {
        self.planning_strategies.contains_key(action_type)
    }
}

impl IntelligentActionPlanner {
    /// Generate alternative actions for high-risk plans
    fn generate_alternatives(
        &self,
        plan: &ActionPlan,
        _context: &ExecutionContext,
    ) -> Result<Vec<AlternativeAction>> {
        let mut alternatives = Vec::new();

        // Generate safer alternatives based on action type
        match plan.action_type {
            ActionType::ToolExecution => {
                alternatives.push(AlternativeAction {
                    action_type: ActionType::Analysis,
                    description: "Analyze the situation before executing tools".to_string(),
                    parameters: HashMap::new(),
                    trigger_conditions: vec!["primary_action_fails".to_string()],
                });
            }
            ActionType::CodeGeneration => {
                alternatives.push(AlternativeAction {
                    action_type: ActionType::Analysis,
                    description: "Review existing code before generating new code".to_string(),
                    parameters: HashMap::new(),
                    trigger_conditions: vec!["generation_fails".to_string()],
                });
            }
            ActionType::FileOperation => {
                alternatives.push(AlternativeAction {
                    action_type: ActionType::Analysis,
                    description: "Analyze file structure before making changes".to_string(),
                    parameters: HashMap::new(),
                    trigger_conditions: vec!["file_operation_fails".to_string()],
                });
            }
            _ => {}
        }

        Ok(alternatives)
    }
}

impl ComprehensiveActionExecutor {
    /// Create a new comprehensive action executor
    pub fn new(
        tool_executor: Box<dyn ToolExecutor>,
        code_generator: Box<dyn CodeGenerator>,
        file_manager: Box<dyn FileManager>,
    ) -> Self {
        Self {
            tool_executor,
            code_generator,
            file_manager,
            capabilities: vec![
                ExecutionCapability::ToolExecution,
                ExecutionCapability::CodeGeneration,
                ExecutionCapability::FileOperations,
                ExecutionCapability::DataAnalysis,
                ExecutionCapability::ErrorRecovery,
            ],
        }
    }
}

#[async_trait]
impl ActionExecutor for ComprehensiveActionExecutor {
    async fn execute(
        &self,
        plan: ActionPlan,
        context: &mut ExecutionContext,
    ) -> Result<ActionResult> {
        let start_time = SystemTime::now();

        let execution_result = match plan.action_type {
            ActionType::ToolExecution => self.execute_tool_action(&plan).await,
            ActionType::CodeGeneration => self.execute_code_generation(&plan, context).await,
            ActionType::FileOperation => self.execute_file_operation(&plan).await,
            ActionType::Analysis => self.execute_analysis(&plan, context).await,
            ActionType::Communication => self.execute_communication(&plan).await,
            ActionType::Planning => self.execute_planning(&plan, context).await,
        };

        let execution_time = start_time.elapsed().unwrap_or_default();

        match execution_result {
            Ok((output, metadata, side_effects)) => Ok(ActionResult {
                action_id: plan.action_id,
                action_type: plan.action_type,
                parameters: plan.parameters,
                result: serde_json::Value::Null,
                execution_time,
                success: true,
                output,
                error: None,
                metadata,
                side_effects,
            }),
            Err(e) => Ok(ActionResult {
                action_id: plan.action_id,
                action_type: plan.action_type,
                parameters: plan.parameters,
                result: serde_json::Value::Null,
                execution_time,
                success: false,
                output: None,
                error: Some(e.to_string()),
                metadata: HashMap::new(),
                side_effects: Vec::new(),
            }),
        }
    }

    fn get_capabilities(&self) -> Vec<ExecutionCapability> {
        self.capabilities.clone()
    }

    fn can_execute(&self, action_type: &ActionType) -> bool {
        matches!(
            action_type,
            ActionType::ToolExecution
                | ActionType::CodeGeneration
                | ActionType::FileOperation
                | ActionType::Analysis
                | ActionType::Communication
                | ActionType::Planning
        )
    }
}

impl ComprehensiveActionExecutor {
    /// Execute tool-based actions
    async fn execute_tool_action(
        &self,
        plan: &ActionPlan,
    ) -> Result<(
        Option<String>,
        HashMap<String, serde_json::Value>,
        Vec<SideEffect>,
    )> {
        let tool_name = plan
            .parameters
            .get("tool_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Tool name not specified in parameters"))?;

        let output = self
            .tool_executor
            .execute_tool(tool_name, &plan.parameters)
            .await?;

        let mut metadata = HashMap::new();
        metadata.insert("tool_used".to_string(), serde_json::json!(tool_name));

        let side_effects = vec![SideEffect {
            effect_type: SideEffectType::ResourceConsumption,
            description: format!("Executed tool: {}", tool_name),
            impact_level: ImpactLevel::Minimal,
        }];

        Ok((Some(output), metadata, side_effects))
    }

    /// Execute code generation actions
    async fn execute_code_generation(
        &self,
        plan: &ActionPlan,
        context: &ExecutionContext,
    ) -> Result<(
        Option<String>,
        HashMap<String, serde_json::Value>,
        Vec<SideEffect>,
    )> {
        let specification = plan
            .parameters
            .get("specification")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Code specification not provided"))?;

        let generated_code = self
            .code_generator
            .generate_code(specification, context)
            .await?;

        let mut metadata = HashMap::new();
        metadata.insert(
            "code_length".to_string(),
            serde_json::json!(generated_code.len()),
        );
        metadata.insert(
            "specification".to_string(),
            serde_json::json!(specification),
        );

        let side_effects = vec![SideEffect {
            effect_type: SideEffectType::StateChange,
            description: "Generated new code".to_string(),
            impact_level: ImpactLevel::Moderate,
        }];

        Ok((Some(generated_code), metadata, side_effects))
    }

    /// Execute file operation actions
    async fn execute_file_operation(
        &self,
        plan: &ActionPlan,
    ) -> Result<(
        Option<String>,
        HashMap<String, serde_json::Value>,
        Vec<SideEffect>,
    )> {
        let operation = plan
            .parameters
            .get("operation")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("File operation not specified"))?;

        let path = plan
            .parameters
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("File path not specified"))?;

        let result = match operation {
            "read" => {
                let content = self.file_manager.read_file(path).await?;
                Some(content)
            }
            "write" => {
                let content = plan
                    .parameters
                    .get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("File content not specified for write operation"))?;
                self.file_manager.write_file(path, content).await?;
                Some(format!("Successfully wrote to {}", path))
            }
            "delete" => {
                self.file_manager.delete_file(path).await?;
                Some(format!("Successfully deleted {}", path))
            }
            _ => return Err(anyhow!("Unsupported file operation: {}", operation)),
        };

        let mut metadata = HashMap::new();
        metadata.insert("operation".to_string(), serde_json::json!(operation));
        metadata.insert("path".to_string(), serde_json::json!(path));

        let side_effects = vec![SideEffect {
            effect_type: SideEffectType::FileModification,
            description: format!("File operation: {} on {}", operation, path),
            impact_level: if operation == "delete" {
                ImpactLevel::Significant
            } else {
                ImpactLevel::Moderate
            },
        }];

        Ok((result, metadata, side_effects))
    }

    /// Execute analysis actions
    async fn execute_analysis(
        &self,
        plan: &ActionPlan,
        _context: &ExecutionContext,
    ) -> Result<(
        Option<String>,
        HashMap<String, serde_json::Value>,
        Vec<SideEffect>,
    )> {
        let analysis_type = plan
            .parameters
            .get("analysis_type")
            .and_then(|v| v.as_str())
            .unwrap_or("general");

        let analysis_result = format!("Analysis completed: {}", analysis_type);

        let mut metadata = HashMap::new();
        metadata.insert(
            "analysis_type".to_string(),
            serde_json::json!(analysis_type),
        );

        Ok((Some(analysis_result), metadata, Vec::new()))
    }

    /// Execute communication actions
    async fn execute_communication(
        &self,
        plan: &ActionPlan,
    ) -> Result<(
        Option<String>,
        HashMap<String, serde_json::Value>,
        Vec<SideEffect>,
    )> {
        let message = plan
            .parameters
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Message not specified for communication"))?;

        // Log the communication using structured logging
        info!("Agent communication: {}", message);

        let mut metadata = HashMap::new();
        metadata.insert(
            "message_length".to_string(),
            serde_json::json!(message.len()),
        );

        Ok((
            Some(format!("Communicated: {}", message)),
            metadata,
            Vec::new(),
        ))
    }

    /// Execute planning actions
    async fn execute_planning(
        &self,
        plan: &ActionPlan,
        _context: &ExecutionContext,
    ) -> Result<(
        Option<String>,
        HashMap<String, serde_json::Value>,
        Vec<SideEffect>,
    )> {
        let planning_scope = plan
            .parameters
            .get("scope")
            .and_then(|v| v.as_str())
            .unwrap_or("general");

        let planning_result = format!("Planning completed for scope: {}", planning_scope);

        let mut metadata = HashMap::new();
        metadata.insert(
            "planning_scope".to_string(),
            serde_json::json!(planning_scope),
        );

        Ok((Some(planning_result), metadata, Vec::new()))
    }
}

// Default planning strategies
struct ToolPlanningStrategy;
struct CodePlanningStrategy;
struct FilePlanningStrategy;
struct AnalysisPlanningStrategy;

#[async_trait]
impl PlanningStrategy for ToolPlanningStrategy {
    async fn plan(
        &self,
        reasoning: &ReasoningResult,
        context: &ExecutionContext,
    ) -> Result<ActionPlan> {
        let output = reasoning.reasoning_output.to_lowercase();

        // Determine which tool to use based on reasoning output
        let (tool_name, description) = if output.contains("shell") || output.contains("command") || output.contains("execute") && output.contains("run") {
            ("run_command", "Execute shell command")
        } else if output.contains("read") && output.contains("file") {
            ("read_file", "Read file contents")
        } else if output.contains("write") && output.contains("file") {
            ("write_file", "Write content to file")
        } else if output.contains("list") && (output.contains("dir") || output.contains("file")) {
            ("list_directory", "List directory contents")
        } else if output.contains("create") && output.contains("dir") {
            ("create_directory", "Create directory")
        } else if output.contains("cargo") || output.contains("rust") && output.contains("build") {
            ("cargo_build", "Build Rust project")
        } else if output.contains("test") && output.contains("rust") {
            ("cargo_test", "Run Rust tests")
        } else {
            // Default to shell command for generic execution requests
            ("run_command", "Execute command")
        };

        let mut parameters = HashMap::new();
        parameters.insert("tool_name".to_string(), serde_json::json!(tool_name));

        // Extract command/path from goal if available
        if let Some(goal) = context.get_current_goal() {
            parameters.insert("goal".to_string(), serde_json::json!(goal.description));
        }

        Ok(ActionPlan {
            action_id: uuid::Uuid::new_v4().to_string(),
            action_type: ActionType::ToolExecution,
            description: format!("{} based on reasoning", description),
            parameters,
            expected_outcome: "Tool execution completed successfully".to_string(),
            confidence_score: reasoning.confidence_score,
            estimated_duration: Some(Duration::from_secs(30)),
            risk_level: RiskLevel::Low,
            alternatives: Vec::new(),
            prerequisites: Vec::new(),
            success_criteria: vec!["Tool executes without errors".to_string()],
        })
    }
}

#[async_trait]
impl PlanningStrategy for CodePlanningStrategy {
    async fn plan(
        &self,
        reasoning: &ReasoningResult,
        context: &ExecutionContext,
    ) -> Result<ActionPlan> {
        let mut parameters = HashMap::new();

        // Use goal description as the specification
        if let Some(goal) = context.get_current_goal() {
            parameters.insert("specification".to_string(), serde_json::json!(goal.description));
        } else {
            // Fallback to reasoning output
            parameters.insert("specification".to_string(), serde_json::json!(reasoning.reasoning_output));
        }

        Ok(ActionPlan {
            action_id: uuid::Uuid::new_v4().to_string(),
            action_type: ActionType::CodeGeneration,
            description: "Generate code based on goal specification".to_string(),
            parameters,
            expected_outcome: "Code generated successfully".to_string(),
            confidence_score: reasoning.confidence_score,
            estimated_duration: Some(Duration::from_secs(60)),
            risk_level: RiskLevel::Medium,
            alternatives: Vec::new(),
            prerequisites: Vec::new(),
            success_criteria: vec![
                "Code compiles successfully".to_string(),
                "Code meets requirements".to_string(),
            ],
        })
    }
}

#[async_trait]
impl PlanningStrategy for FilePlanningStrategy {
    async fn plan(
        &self,
        reasoning: &ReasoningResult,
        context: &ExecutionContext,
    ) -> Result<ActionPlan> {
        let output = reasoning.reasoning_output.to_lowercase();

        // Determine file operation type
        let operation = if output.contains("read") {
            "read"
        } else if output.contains("write") || output.contains("create") || output.contains("save") {
            "write"
        } else if output.contains("delete") || output.contains("remove") {
            "delete"
        } else if output.contains("list") || output.contains("dir") {
            "list"
        } else {
            "read" // Default to read as it's safe
        };

        let mut parameters = HashMap::new();
        parameters.insert("operation".to_string(), serde_json::json!(operation));

        // Include goal for path extraction
        if let Some(goal) = context.get_current_goal() {
            parameters.insert("goal".to_string(), serde_json::json!(goal.description));
        }

        Ok(ActionPlan {
            action_id: uuid::Uuid::new_v4().to_string(),
            action_type: ActionType::FileOperation,
            description: format!("Perform {} file operation", operation),
            parameters,
            expected_outcome: "File operation completed successfully".to_string(),
            confidence_score: reasoning.confidence_score,
            estimated_duration: Some(Duration::from_secs(10)),
            risk_level: RiskLevel::Low,
            alternatives: Vec::new(),
            prerequisites: Vec::new(),
            success_criteria: vec!["File operation succeeds".to_string()],
        })
    }
}

#[async_trait]
impl PlanningStrategy for AnalysisPlanningStrategy {
    async fn plan(
        &self,
        reasoning: &ReasoningResult,
        _context: &ExecutionContext,
    ) -> Result<ActionPlan> {
        Ok(ActionPlan {
            action_id: uuid::Uuid::new_v4().to_string(),
            action_type: ActionType::Analysis,
            description: "Perform analysis based on reasoning".to_string(),
            parameters: HashMap::new(),
            expected_outcome: "Analysis completed with insights".to_string(),
            confidence_score: reasoning.confidence_score,
            estimated_duration: Some(Duration::from_secs(45)),
            risk_level: RiskLevel::Low,
            alternatives: Vec::new(),
            prerequisites: Vec::new(),
            success_criteria: vec!["Analysis provides useful insights".to_string()],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_plan_creation() {
        let plan = ActionPlan {
            action_id: "test-id".to_string(),
            action_type: ActionType::ToolExecution,
            description: "Test action".to_string(),
            parameters: HashMap::new(),
            expected_outcome: "Success".to_string(),
            confidence_score: 0.8,
            estimated_duration: Some(Duration::from_secs(30)),
            risk_level: RiskLevel::Low,
            alternatives: Vec::new(),
            prerequisites: Vec::new(),
            success_criteria: Vec::new(),
        };

        assert_eq!(plan.action_id, "test-id");
        assert_eq!(plan.confidence_score, 0.8);
        assert!(matches!(plan.action_type, ActionType::ToolExecution));
    }

    #[test]
    fn test_risk_level_ordering() {
        assert!(matches!(RiskLevel::Low, RiskLevel::Low));
        assert!(matches!(RiskLevel::Critical, RiskLevel::Critical));
    }
}
