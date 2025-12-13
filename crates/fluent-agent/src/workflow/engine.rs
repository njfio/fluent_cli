use super::template::TemplateEngine;
use super::{
    utils, BackoffStrategy, RetryConfig, StepResult, StepStatus, WorkflowContext,
    WorkflowDefinition, WorkflowResult, WorkflowStatus,
};
use crate::tools::ToolRegistry;
use anyhow::Result;
use petgraph::graph::NodeIndex;
use petgraph::{Direction, Graph};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::Semaphore;
use tokio::time::timeout;
use tracing::warn;
use uuid::Uuid;

/// Workflow execution engine with DAG-based execution
#[allow(dead_code)]
pub struct WorkflowEngine {
    tool_registry: Arc<ToolRegistry>,
    max_concurrent_steps: usize,
    semaphore: Arc<Semaphore>,
}

impl WorkflowEngine {
    pub fn new(tool_registry: Arc<ToolRegistry>, max_concurrent_steps: usize) -> Self {
        Self {
            tool_registry,
            max_concurrent_steps,
            semaphore: Arc::new(Semaphore::new(max_concurrent_steps)),
        }
    }

    /// Execute a workflow definition
    pub async fn execute_workflow(
        &self,
        definition: WorkflowDefinition,
        inputs: HashMap<String, serde_json::Value>,
    ) -> Result<WorkflowResult> {
        // Validate workflow definition
        utils::validate_workflow_definition(&definition)?;

        // Create execution context
        let execution_id = Uuid::new_v4().to_string();
        let mut context =
            WorkflowContext::new(definition.name.clone(), execution_id.clone(), inputs);

        // Build execution DAG
        let dag = self.build_execution_dag(&definition)?;

        // Execute workflow
        let start_time = SystemTime::now();
        let result = self.execute_dag(dag, &mut context, &definition).await;
        let end_time = SystemTime::now();

        // Build workflow result
        let workflow_result = WorkflowResult {
            workflow_id: definition.name.clone(),
            execution_id,
            status: match result {
                Ok(_) => WorkflowStatus::Completed,
                Err(_) => WorkflowStatus::Failed,
            },
            outputs: self.extract_outputs(&context, &definition),
            step_results: self.build_step_results(&context),
            start_time,
            end_time,
            duration: end_time.duration_since(start_time).unwrap_or_default(),
            error: result.err().map(|e| e.to_string()),
            metadata: context.metadata.clone(),
        };

        Ok(workflow_result)
    }

    /// Build a DAG from workflow definition
    fn build_execution_dag(&self, definition: &WorkflowDefinition) -> Result<Graph<String, ()>> {
        let mut graph = Graph::new();
        let mut node_map = HashMap::new();

        // Add all steps as nodes
        for step in &definition.steps {
            let node_index = graph.add_node(step.id.clone());
            node_map.insert(step.id.clone(), node_index);
        }

        // Add dependency edges
        for step in &definition.steps {
            if let Some(ref dependencies) = step.depends_on {
                for dep in dependencies {
                    if let (Some(&dep_node), Some(&step_node)) =
                        (node_map.get(dep), node_map.get(&step.id))
                    {
                        graph.add_edge(dep_node, step_node, ());
                    }
                }
            }
        }

        // Validate DAG (no cycles)
        if petgraph::algo::is_cyclic_directed(&graph) {
            return Err(anyhow::anyhow!("Workflow contains circular dependencies"));
        }

        Ok(graph)
    }

    /// Execute the DAG
    async fn execute_dag(
        &self,
        graph: Graph<String, ()>,
        context: &mut WorkflowContext,
        definition: &WorkflowDefinition,
    ) -> Result<()> {
        let step_map: HashMap<String, &super::WorkflowStep> = definition
            .steps
            .iter()
            .map(|step| (step.id.clone(), step))
            .collect();

        // Find nodes with no incoming edges (starting points)
        let mut ready_queue = VecDeque::new();
        let mut in_degree = HashMap::new();

        for node_index in graph.node_indices() {
            let _step_id = &graph[node_index];
            let degree = graph
                .neighbors_directed(node_index, Direction::Incoming)
                .count();
            in_degree.insert(node_index, degree);

            if degree == 0 {
                ready_queue.push_back(node_index);
            }
        }

        // Execute steps in topological order
        while !ready_queue.is_empty() {
            // Execute steps one by one for now (simplified implementation)
            if let Some(node_index) = ready_queue.pop_front() {
                let step_id = &graph[node_index];
                let step = step_map
                    .get(step_id)
                    .ok_or_else(|| anyhow::anyhow!("Step not found in step map: {}", step_id))?;

                // Execute single step
                self.execute_step(step, context).await?;

                // Update ready queue
                self.update_ready_queue(&graph, node_index, &mut ready_queue, &mut in_degree);
            }
        }

        Ok(())
    }

    /// Update the ready queue after step completion
    fn update_ready_queue(
        &self,
        graph: &Graph<String, ()>,
        completed_node: NodeIndex,
        ready_queue: &mut VecDeque<NodeIndex>,
        in_degree: &mut HashMap<NodeIndex, usize>,
    ) {
        // Decrease in-degree for all dependent steps
        for neighbor in graph.neighbors_directed(completed_node, Direction::Outgoing) {
            if let Some(degree) = in_degree.get_mut(&neighbor) {
                *degree -= 1;
                if *degree == 0 {
                    ready_queue.push_back(neighbor);
                }
            }
        }
    }

    /// Execute a single step
    async fn execute_step(
        &self,
        step: &super::WorkflowStep,
        context: &mut WorkflowContext,
    ) -> Result<()> {
        Self::execute_step_impl(&self.tool_registry, step, context).await
    }

    /// Internal step execution implementation
    async fn execute_step_impl(
        tool_registry: &ToolRegistry,
        step: &super::WorkflowStep,
        context: &mut WorkflowContext,
    ) -> Result<()> {
        // Start timing for this step
        context.start_step_timing(&step.id);
        context.set_step_status(&step.id, StepStatus::Running);

        // Check condition if specified
        if let Some(ref condition) = step.condition {
            if !Self::evaluate_condition(condition, context)? {
                context.set_step_status(&step.id, StepStatus::Skipped);
                return Ok(());
            }
        }

        // Resolve parameters with template engine
        let resolved_params = Self::resolve_parameters(&step.parameters, context)?;

        // Execute with retry logic
        let default_retry = RetryConfig::default();
        let retry_config = step.retry.as_ref().unwrap_or(&default_retry);
        let result = Self::execute_with_retry(
            tool_registry,
            &step.tool,
            &resolved_params,
            retry_config,
            step.timeout.as_deref(),
            &step.id,
            context,
        )
        .await;

        match result {
            Ok(output) => {
                // Store step outputs
                if let Some(ref output_mapping) = step.outputs {
                    for (key, path) in output_mapping {
                        // Extract value from output using path
                        let value = Self::extract_value_by_path(&output, path)?;
                        context.set_step_output(&step.id, key, value);
                    }
                } else {
                    // Store entire output
                    context.set_step_output(&step.id, "result", output);
                }

                context.set_step_status(&step.id, StepStatus::Completed);
                context.end_step_timing(&step.id);
            }
            Err(e) => {
                context.set_step_status(
                    &step.id,
                    StepStatus::Failed {
                        error: e.to_string(),
                        attempt: retry_config.max_attempts,
                    },
                );
                context.end_step_timing(&step.id);

                // Handle error based on step configuration
                match step.on_error.as_ref() {
                    Some(super::ErrorAction::Continue) => {
                        // Continue execution despite error
                    }
                    Some(super::ErrorAction::Skip) => {
                        context.set_step_status(&step.id, StepStatus::Skipped);
                    }
                    _ => {
                        return Err(e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Execute tool with retry logic
    async fn execute_with_retry(
        tool_registry: &ToolRegistry,
        tool_name: &str,
        parameters: &HashMap<String, serde_json::Value>,
        retry_config: &RetryConfig,
        timeout_str: Option<&str>,
        step_id: &str,
        context: &mut WorkflowContext,
    ) -> Result<serde_json::Value> {
        let timeout_duration = if let Some(timeout_str) = timeout_str {
            Some(utils::parse_duration(timeout_str)?)
        } else {
            None
        };

        let mut attempts = 0;
        let mut delay = match &retry_config.backoff {
            BackoffStrategy::Fixed { delay } => utils::parse_duration(delay)?,
            BackoffStrategy::Exponential { initial_delay, .. } => {
                utils::parse_duration(initial_delay)?
            }
            BackoffStrategy::Linear { increment } => utils::parse_duration(increment)?,
        };

        loop {
            attempts += 1;
            context.increment_step_attempts(step_id);

            let execution_future = tool_registry.execute_tool(tool_name, parameters);

            let result = if let Some(timeout_duration) = timeout_duration {
                timeout(timeout_duration, execution_future).await?
            } else {
                execution_future.await
            };

            match result {
                Ok(output) => {
                    let value = serde_json::from_str(&output)?;
                    return Ok(value);
                }
                Err(error) => {
                    if attempts >= retry_config.max_attempts {
                        return Err(error);
                    }

                    // Check if error is retryable
                    let error_str = error.to_string().to_lowercase();
                    let should_retry = retry_config
                        .retry_on
                        .as_ref()
                        .map(|retry_errors| {
                            retry_errors
                                .iter()
                                .any(|retry_error| error_str.contains(retry_error))
                        })
                        .unwrap_or(true);

                    if !should_retry {
                        return Err(error);
                    }

                    // Wait before retry
                    tokio::time::sleep(delay).await;

                    // Calculate next delay
                    match &retry_config.backoff {
                        BackoffStrategy::Fixed { .. } => {
                            // delay stays the same
                        }
                        BackoffStrategy::Exponential { max_delay, .. } => {
                            let max_delay = utils::parse_duration(max_delay)?;
                            delay = (delay * 2).min(max_delay);
                        }
                        BackoffStrategy::Linear { increment } => {
                            let increment = utils::parse_duration(increment)?;
                            delay += increment;
                        }
                    }
                }
            }
        }
    }

    /// Resolve template parameters
    fn resolve_parameters(
        parameters: &HashMap<String, serde_json::Value>,
        context: &WorkflowContext,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let template_engine = TemplateEngine::new();
        template_engine.resolve_parameters(parameters, context)
    }

    /// Evaluate step condition
    fn evaluate_condition(condition: &str, context: &WorkflowContext) -> Result<bool> {
        // Simple condition evaluation supporting basic expressions
        let condition = condition.trim();

        // Handle empty conditions
        if condition.is_empty() {
            return Ok(true);
        }

        // Handle boolean literals
        if condition == "true" {
            return Ok(true);
        }
        if condition == "false" {
            return Ok(false);
        }

        // Handle variable references like ${variable_name}
        if condition.starts_with("${") && condition.ends_with("}") {
            let var_name = &condition[2..condition.len() - 1];
            if let Some(value) = context.variables.get(var_name) {
                return match value {
                    serde_json::Value::Bool(b) => Ok(*b),
                    serde_json::Value::String(s) => Ok(!s.is_empty()),
                    serde_json::Value::Number(n) => Ok(n.as_f64().unwrap_or(0.0) != 0.0),
                    serde_json::Value::Null => Ok(false),
                    _ => Ok(true),
                };
            }
            return Ok(false);
        }

        // Handle simple comparisons like "variable == value"
        if condition.contains("==") {
            let parts: Vec<&str> = condition.split("==").map(|s| s.trim()).collect();
            if parts.len() == 2 {
                let left = Self::resolve_value(parts[0], context)?;
                let right = Self::resolve_value(parts[1], context)?;
                return Ok(left == right);
            }
        }

        if condition.contains("!=") {
            let parts: Vec<&str> = condition.split("!=").map(|s| s.trim()).collect();
            if parts.len() == 2 {
                let left = Self::resolve_value(parts[0], context)?;
                let right = Self::resolve_value(parts[1], context)?;
                return Ok(left != right);
            }
        }

        // Default to true for unknown conditions
        warn!("Unknown condition format: {}", condition);
        Ok(true)
    }

    /// Resolve a value from context or return as literal
    fn resolve_value(value: &str, context: &WorkflowContext) -> Result<serde_json::Value> {
        let value = value.trim();

        // Handle variable references
        if value.starts_with("${") && value.ends_with("}") {
            let var_name = &value[2..value.len() - 1];
            return Ok(context
                .variables
                .get(var_name)
                .cloned()
                .unwrap_or(serde_json::Value::Null));
        }

        // Handle string literals
        if (value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\''))
        {
            return Ok(serde_json::Value::String(
                value[1..value.len() - 1].to_string(),
            ));
        }

        // Handle number literals
        if let Ok(n) = value.parse::<f64>() {
            if let Some(number) = serde_json::Number::from_f64(n) {
                return Ok(serde_json::Value::Number(number));
            } else {
                return Err(anyhow::anyhow!("Invalid number value: {}", n));
            }
        }

        // Handle boolean literals
        if value == "true" {
            return Ok(serde_json::Value::Bool(true));
        }
        if value == "false" {
            return Ok(serde_json::Value::Bool(false));
        }

        // Default to string
        Ok(serde_json::Value::String(value.to_string()))
    }

    /// Extract value from output using JSONPath-like syntax
    fn extract_value_by_path(output: &serde_json::Value, path: &str) -> Result<serde_json::Value> {
        let path = path.trim();

        // Handle empty path - return entire output
        if path.is_empty() || path == "$" {
            return Ok(output.clone());
        }

        // Remove leading $ if present
        let path = path.strip_prefix('$').unwrap_or(path);

        // Handle simple dot notation like .field or .field.subfield
        if let Some(path) = path.strip_prefix('.') {
            return Self::extract_by_dot_notation(output, path);
        }

        // Handle array access like [0] or field[0]
        if path.contains('[') && path.contains(']') {
            return Self::extract_with_array_access(output, path);
        }

        // Handle simple field access
        if let serde_json::Value::Object(obj) = output {
            if let Some(value) = obj.get(path) {
                return Ok(value.clone());
            }
        }

        // Path not found
        Ok(serde_json::Value::Null)
    }

    /// Extract value using dot notation
    fn extract_by_dot_notation(
        mut current: &serde_json::Value,
        path: &str,
    ) -> Result<serde_json::Value> {
        if path.is_empty() {
            return Ok(current.clone());
        }

        let parts: Vec<&str> = path.split('.').collect();

        for part in parts {
            if part.is_empty() {
                continue;
            }

            // Handle array access in part like "field[0]"
            if part.contains('[') && part.ends_with(']') {
                let bracket_pos = part
                    .find('[')
                    .ok_or_else(|| anyhow::anyhow!("Invalid array access syntax: {}", part))?;
                let field_name = &part[..bracket_pos];
                let index_str = &part[bracket_pos + 1..part.len() - 1];

                // First access the field
                if !field_name.is_empty() {
                    if let serde_json::Value::Object(obj) = current {
                        if let Some(field_value) = obj.get(field_name) {
                            current = field_value;
                        } else {
                            return Ok(serde_json::Value::Null);
                        }
                    } else {
                        return Ok(serde_json::Value::Null);
                    }
                }

                // Then access the array index
                if let Ok(index) = index_str.parse::<usize>() {
                    if let serde_json::Value::Array(arr) = current {
                        if index < arr.len() {
                            current = &arr[index];
                        } else {
                            return Ok(serde_json::Value::Null);
                        }
                    } else {
                        return Ok(serde_json::Value::Null);
                    }
                } else {
                    return Ok(serde_json::Value::Null);
                }
            } else {
                // Simple field access
                if let serde_json::Value::Object(obj) = current {
                    if let Some(field_value) = obj.get(part) {
                        current = field_value;
                    } else {
                        return Ok(serde_json::Value::Null);
                    }
                } else {
                    return Ok(serde_json::Value::Null);
                }
            }
        }

        Ok(current.clone())
    }

    /// Extract value with array access
    fn extract_with_array_access(
        output: &serde_json::Value,
        path: &str,
    ) -> Result<serde_json::Value> {
        // Simple implementation for paths like [0] or field[0]
        if path.starts_with('[') && path.ends_with(']') {
            // Direct array access like [0]
            let index_str = &path[1..path.len() - 1];
            if let Ok(index) = index_str.parse::<usize>() {
                if let serde_json::Value::Array(arr) = output {
                    if index < arr.len() {
                        return Ok(arr[index].clone());
                    }
                }
            }
            return Ok(serde_json::Value::Null);
        }

        // Field with array access like field[0]
        let bracket_pos = path
            .find('[')
            .ok_or_else(|| anyhow::anyhow!("Invalid array access syntax in path: {}", path))?;
        let field_name = &path[..bracket_pos];
        let index_part = &path[bracket_pos..];

        // First get the field
        let field_value = if field_name.is_empty() {
            output
        } else if let serde_json::Value::Object(obj) = output {
            obj.get(field_name).unwrap_or(&serde_json::Value::Null)
        } else {
            &serde_json::Value::Null
        };

        // Then apply array access
        Self::extract_with_array_access(field_value, index_part)
    }

    /// Extract workflow outputs from context
    fn extract_outputs(
        &self,
        context: &WorkflowContext,
        definition: &WorkflowDefinition,
    ) -> HashMap<String, serde_json::Value> {
        let mut outputs = HashMap::new();

        for output_def in &definition.outputs {
            // Try to find the output value in different places
            let value =
                // First, try to find in variables by exact name
                context.variables.get(&output_def.name).cloned()
                .or_else(|| {
                    // Try to find in the last step's output if it matches the output name
                    if let Some(last_step_id) = definition.steps.last().map(|s| &s.id) {
                        context.step_outputs.get(last_step_id)
                            .and_then(|output_map| {
                                // Try to extract field with the same name as output
                                output_map.get(&output_def.name).cloned()
                            })
                    } else {
                        None
                    }
                })
                .or_else(|| {
                    // Try to find in any step output that has a field with this name
                    for step_output_map in context.step_outputs.values() {
                        if let Some(field_value) = step_output_map.get(&output_def.name) {
                            return Some(field_value.clone());
                        }
                    }
                    None
                })
                .unwrap_or(serde_json::Value::Null);

            outputs.insert(output_def.name.clone(), value);
        }

        outputs
    }

    /// Build step results from context
    fn build_step_results(&self, context: &WorkflowContext) -> HashMap<String, StepResult> {
        let mut results = HashMap::new();

        for (step_id, status) in &context.step_status {
            let result = StepResult {
                step_id: step_id.clone(),
                status: status.clone(),
                outputs: context
                    .step_outputs
                    .get(step_id)
                    .cloned()
                    .unwrap_or_default(),
                start_time: context
                    .step_start_times
                    .get(step_id)
                    .copied()
                    .unwrap_or(context.start_time),
                end_time: context.step_end_times.get(step_id).copied(),
                duration: context.get_step_duration(step_id),
                error: match status {
                    StepStatus::Failed { error, .. } => Some(error.clone()),
                    _ => None,
                },
                attempts: context.get_step_attempts(step_id),
            };

            results.insert(step_id.clone(), result);
        }

        results
    }
}

// ============================================================================
// ExecutionLoop Implementation for WorkflowEngine
// ============================================================================

use crate::execution::{
    ExecutionLoop, ExecutionState, ExecutionStatus, StepResult as ExecStepResult,
};
use std::time::Duration;

/// Adapter to run WorkflowEngine through the unified ExecutionLoop interface
///
/// This adapter wraps a WorkflowEngine and exposes its DAG-based execution as
/// discrete steps that can be controlled by the UniversalExecutor.
pub struct WorkflowExecutionAdapter {
    /// The workflow engine
    engine: WorkflowEngine,
    /// Workflow definition to execute
    definition: WorkflowDefinition,
    /// Input parameters
    inputs: HashMap<String, serde_json::Value>,
    /// Execution context
    context: WorkflowContext,
    /// Execution DAG
    dag: Option<Graph<String, ()>>,
    /// Queue of ready steps (no pending dependencies)
    ready_queue: VecDeque<NodeIndex>,
    /// In-degree for each node (dependencies remaining)
    in_degree: HashMap<NodeIndex, usize>,
    /// Map of step IDs to their definitions
    step_map: HashMap<String, super::WorkflowStep>,
    /// Unified execution state
    execution_state: ExecutionState,
    /// Whether initialization has been called
    initialized: bool,
    /// Last error encountered
    last_error: Option<String>,
    /// Start time
    start_time: std::time::Instant,
    /// Total steps to execute
    total_steps: usize,
    /// Steps completed
    steps_completed: usize,
}

impl WorkflowExecutionAdapter {
    /// Create a new adapter for running a workflow
    pub fn new(
        engine: WorkflowEngine,
        definition: WorkflowDefinition,
        inputs: HashMap<String, serde_json::Value>,
    ) -> Self {
        let total_steps = definition.steps.len();
        let execution_id = Uuid::new_v4().to_string();
        let context = WorkflowContext::new(definition.name.clone(), execution_id, inputs.clone());

        Self {
            engine,
            definition,
            inputs,
            context,
            dag: None,
            ready_queue: VecDeque::new(),
            in_degree: HashMap::new(),
            step_map: HashMap::new(),
            execution_state: ExecutionState::new(Some(total_steps as u32)),
            initialized: false,
            last_error: None,
            start_time: std::time::Instant::now(),
            total_steps,
            steps_completed: 0,
        }
    }

    /// Get the workflow result after execution
    pub fn get_result(&self) -> WorkflowResult {
        let end_time = SystemTime::now();
        WorkflowResult {
            workflow_id: self.definition.name.clone(),
            execution_id: self.context.execution_id.clone(),
            status: match self.execution_state.status {
                ExecutionStatus::Completed => WorkflowStatus::Completed,
                ExecutionStatus::Failed => WorkflowStatus::Failed,
                ExecutionStatus::Terminated => WorkflowStatus::Cancelled,
                _ => WorkflowStatus::Running,
            },
            outputs: self.engine.extract_outputs(&self.context, &self.definition),
            step_results: self.engine.build_step_results(&self.context),
            start_time: self.context.start_time,
            end_time,
            duration: end_time
                .duration_since(self.context.start_time)
                .unwrap_or_default(),
            error: self.last_error.clone(),
            metadata: self.context.metadata.clone(),
        }
    }
}

#[async_trait::async_trait]
impl ExecutionLoop for WorkflowExecutionAdapter {
    type State = ExecutionState;

    async fn initialize(&mut self) -> Result<()> {
        if self.initialized {
            return Ok(());
        }

        // Validate workflow definition
        utils::validate_workflow_definition(&self.definition)?;

        // Build step map
        self.step_map = self
            .definition
            .steps
            .iter()
            .cloned()
            .map(|step| (step.id.clone(), step))
            .collect();

        // Build execution DAG
        let dag = self.engine.build_execution_dag(&self.definition)?;

        // Initialize ready queue with steps that have no dependencies
        for node_index in dag.node_indices() {
            let degree = dag
                .neighbors_directed(node_index, Direction::Incoming)
                .count();
            self.in_degree.insert(node_index, degree);

            if degree == 0 {
                self.ready_queue.push_back(node_index);
            }
        }

        self.dag = Some(dag);
        self.execution_state.status = ExecutionStatus::Running;
        self.initialized = true;

        tracing::info!(
            "execution_loop.workflow.init workflow='{}' steps={}",
            self.definition.name,
            self.total_steps
        );

        Ok(())
    }

    async fn execute_step(&mut self) -> Result<ExecStepResult> {
        let step_start = std::time::Instant::now();
        self.execution_state.next_iteration();

        // Get next ready step
        let node_index = match self.ready_queue.pop_front() {
            Some(idx) => idx,
            None => {
                return Ok(ExecStepResult::failure(
                    "no-ready-step",
                    "No steps ready to execute",
                    step_start.elapsed(),
                ));
            }
        };

        let dag = self
            .dag
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("DAG not initialized"))?;
        let step_id = dag[node_index].clone();

        self.execution_state.current_step = step_id.clone();

        tracing::debug!(
            "execution_loop.workflow.step iter={} step={}",
            self.execution_state.iteration,
            step_id
        );

        let step = self
            .step_map
            .get(&step_id)
            .ok_or_else(|| anyhow::anyhow!("Step not found: {}", step_id))?
            .clone();

        // Execute the step
        let result =
            WorkflowEngine::execute_step_impl(&self.engine.tool_registry, &step, &mut self.context)
                .await;

        // Update ready queue with newly unblocked steps
        if let Some(dag) = &self.dag {
            for neighbor in dag.neighbors_directed(node_index, Direction::Outgoing) {
                if let Some(degree) = self.in_degree.get_mut(&neighbor) {
                    *degree = degree.saturating_sub(1);
                    if *degree == 0 {
                        self.ready_queue.push_back(neighbor);
                    }
                }
            }
        }

        self.steps_completed += 1;

        // Add observation
        let status_str = match self.context.step_status.get(&step_id) {
            Some(StepStatus::Completed) => "completed",
            Some(StepStatus::Skipped) => "skipped",
            Some(StepStatus::Failed { .. }) => "failed",
            _ => "unknown",
        };
        self.execution_state
            .add_observation(format!("[workflow] Step {} {}", step_id, status_str), 10);

        match result {
            Ok(()) => Ok(ExecStepResult::success(
                step_id,
                format!("Step completed (status: {})", status_str),
                step_start.elapsed(),
            )),
            Err(e) => {
                self.last_error = Some(e.to_string());
                Ok(ExecStepResult::failure(
                    step_id,
                    e.to_string(),
                    step_start.elapsed(),
                ))
            }
        }
    }

    fn current_step_id(&self) -> String {
        self.execution_state.current_step.clone()
    }

    fn should_continue(&self) -> bool {
        // Continue if there are steps ready to execute
        !self.ready_queue.is_empty()
            && !matches!(
                self.execution_state.status,
                ExecutionStatus::Completed | ExecutionStatus::Failed | ExecutionStatus::Terminated
            )
    }

    fn is_retryable_error(&self) -> bool {
        self.last_error.is_some()
    }

    fn is_complete(&self) -> Result<bool> {
        // Complete when all steps are done and no errors
        let all_done = self.steps_completed >= self.total_steps || self.ready_queue.is_empty();
        let no_failures = !self
            .context
            .step_status
            .values()
            .any(|s| matches!(s, StepStatus::Failed { .. }));

        Ok(all_done && no_failures)
    }

    fn should_terminate(&self) -> Result<bool> {
        // Check for timeout (default 1 hour for workflows)
        let timeout_duration = Duration::from_secs(60 * 60);
        if self.start_time.elapsed() > timeout_duration {
            return Ok(true);
        }

        // Check for max iterations
        if self.execution_state.is_max_iterations_exceeded() {
            return Ok(true);
        }

        // Terminate if a step failed and error handling is not set to continue
        if let Some(ref error) = self.last_error {
            // Check if we should fail fast
            if self.definition.error_handling.is_none() {
                tracing::warn!("execution_loop.workflow.fail_fast error={}", error);
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn get_state(&self) -> &Self::State {
        &self.execution_state
    }

    fn get_state_mut(&mut self) -> &mut Self::State {
        &mut self.execution_state
    }

    async fn save_checkpoint(&self) -> Result<String> {
        // Workflows don't support checkpointing in this implementation
        Ok(format!(
            "workflow-{}-{}",
            self.context.execution_id, self.execution_state.iteration
        ))
    }

    async fn restore_checkpoint(&mut self, _id: &str) -> Result<()> {
        // Not implemented for workflows
        Ok(())
    }

    fn iteration(&self) -> u32 {
        self.execution_state.iteration
    }

    fn max_iterations(&self) -> Option<u32> {
        self.execution_state.max_iterations
    }

    fn elapsed_time(&self) -> Duration {
        self.start_time.elapsed()
    }

    async fn handle_error(&mut self, error: anyhow::Error) -> Result<()> {
        self.last_error = Some(error.to_string());
        self.execution_state.error_count += 1;

        tracing::warn!(
            "execution_loop.workflow.error iter={} step={} error={}",
            self.execution_state.iteration,
            self.execution_state.current_step,
            error
        );

        Ok(())
    }

    fn reset_error_state(&mut self) {
        self.last_error = None;
    }

    fn get_metrics(&self) -> serde_json::Value {
        serde_json::json!({
            "workflow": self.definition.name,
            "execution_id": self.context.execution_id,
            "total_steps": self.total_steps,
            "steps_completed": self.steps_completed,
            "steps_remaining": self.ready_queue.len(),
            "iteration": self.execution_state.iteration,
            "error_count": self.execution_state.error_count,
            "elapsed_ms": self.start_time.elapsed().as_millis(),
            "status": format!("{:?}", self.execution_state.status),
        })
    }

    fn get_recent_observations(&self, n: usize) -> Vec<String> {
        self.execution_state
            .recent_observations
            .iter()
            .rev()
            .take(n)
            .cloned()
            .collect()
    }
}

// ============================================================================
// End ExecutionLoop Implementation
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::ToolRegistry;
    use crate::workflow::WorkflowStep;

    #[tokio::test]
    async fn test_workflow_engine_creation() {
        let tool_registry = Arc::new(ToolRegistry::new());
        let engine = WorkflowEngine::new(tool_registry, 5);

        assert_eq!(engine.max_concurrent_steps, 5);
    }

    #[test]
    fn test_dag_building() {
        let tool_registry = Arc::new(ToolRegistry::new());
        let engine = WorkflowEngine::new(tool_registry, 5);

        let definition = WorkflowDefinition {
            name: "test_workflow".to_string(),
            version: "1.0".to_string(),
            description: None,
            inputs: vec![],
            outputs: vec![],
            steps: vec![
                WorkflowStep {
                    id: "step1".to_string(),
                    name: None,
                    tool: "test_tool".to_string(),
                    parameters: HashMap::new(),
                    depends_on: None,
                    outputs: None,
                    retry: None,
                    timeout: None,
                    parallel: None,
                    condition: None,
                    on_error: None,
                },
                WorkflowStep {
                    id: "step2".to_string(),
                    name: None,
                    tool: "test_tool2".to_string(),
                    parameters: HashMap::new(),
                    depends_on: Some(vec!["step1".to_string()]),
                    outputs: None,
                    retry: None,
                    timeout: None,
                    parallel: None,
                    condition: None,
                    on_error: None,
                },
            ],
            error_handling: None,
            metadata: None,
        };

        let dag = engine.build_execution_dag(&definition).unwrap();
        assert_eq!(dag.node_count(), 2);
        assert_eq!(dag.edge_count(), 1);
    }
}
