# Advanced Tool Composition and Chaining Implementation Plan

## Executive Summary

This document outlines the implementation plan for advanced tool composition and chaining capabilities in Fluent CLI's agentic system. The plan includes workflow orchestration, declarative tool composition, dependency resolution, and parallel execution frameworks.

## Current State Analysis

### Existing Tool System
- **File**: `crates/fluent-agent/src/tools/mod.rs`
- **Architecture**: Simple registry-based tool execution
- **Limitations**:
  - No tool chaining or composition
  - Sequential execution only
  - No dependency management
  - Limited error handling and retry logic
  - No workflow persistence

### Current Tool Registry
```rust
pub struct ToolRegistry {
    executors: HashMap<String, Arc<dyn ToolExecutor>>,
}

// Simple execution pattern
pub async fn execute_tool(&self, tool_name: &str, parameters: &HashMap<String, serde_json::Value>) -> Result<String>
```

## Technical Research Summary

### Workflow Orchestration Patterns
1. **DAG-based Execution**: Directed Acyclic Graph for dependency resolution
2. **Pipeline Patterns**: Linear and branching execution flows
3. **Event-driven Architecture**: Reactive tool execution based on events
4. **State Machines**: Complex workflow state management

### Industry Examples Analysis
- **Temporal**: Durable workflow execution with compensation
- **Airflow**: DAG-based task orchestration
- **Prefect**: Modern workflow orchestration with dynamic flows
- **GitHub Actions**: YAML-based workflow definition

### Rust Workflow Libraries
- **`tokio`**: Async runtime and task management
- **`petgraph`**: Graph data structures for DAG representation
- **`serde_yaml`**: YAML parsing for workflow definitions
- **`futures`**: Stream processing and combinators

## Implementation Plan

### Phase 1: Workflow Definition Framework (3-4 weeks)

#### 1.1 Declarative Workflow Schema
```yaml
# workflow_example.yaml
name: "code_analysis_workflow"
version: "1.0"
description: "Comprehensive code analysis and improvement workflow"

inputs:
  - name: "project_path"
    type: "string"
    required: true
  - name: "language"
    type: "string"
    default: "rust"

outputs:
  - name: "analysis_report"
    type: "string"
  - name: "improvement_suggestions"
    type: "array"

steps:
  - id: "read_files"
    tool: "filesystem.list_files"
    parameters:
      path: "{{ inputs.project_path }}"
      pattern: "**/*.rs"
    outputs:
      files: "result"

  - id: "analyze_code"
    tool: "rust_compiler.check"
    depends_on: ["read_files"]
    parameters:
      files: "{{ steps.read_files.outputs.files }}"
    retry:
      max_attempts: 3
      backoff: "exponential"
    timeout: "5m"

  - id: "security_scan"
    tool: "security.audit"
    depends_on: ["read_files"]
    parameters:
      path: "{{ inputs.project_path }}"
    parallel: true

  - id: "generate_report"
    tool: "reporting.create_markdown"
    depends_on: ["analyze_code", "security_scan"]
    parameters:
      template: "code_analysis_template.md"
      data:
        analysis: "{{ steps.analyze_code.outputs }}"
        security: "{{ steps.security_scan.outputs }}"
```

#### 1.2 Workflow Definition Structures
```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub inputs: Vec<WorkflowInput>,
    pub outputs: Vec<WorkflowOutput>,
    pub steps: Vec<WorkflowStep>,
    pub error_handling: Option<ErrorHandlingConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub id: String,
    pub tool: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub depends_on: Option<Vec<String>>,
    pub outputs: Option<HashMap<String, String>>,
    pub retry: Option<RetryConfig>,
    pub timeout: Option<String>,
    pub parallel: Option<bool>,
    pub condition: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub backoff: BackoffStrategy,
    pub retry_on: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum BackoffStrategy {
    Fixed { delay: String },
    Exponential { initial_delay: String, max_delay: String },
    Linear { increment: String },
}
```

### Phase 2: Workflow Execution Engine (4-5 weeks)

#### 2.1 DAG-based Execution Engine
```rust
use petgraph::{Graph, Direction};
use petgraph::graph::NodeIndex;
use std::collections::{HashMap, VecDeque};

pub struct WorkflowExecutor {
    tool_registry: Arc<ToolRegistry>,
    execution_context: Arc<RwLock<ExecutionContext>>,
    metrics_collector: Arc<MetricsCollector>,
}

pub struct ExecutionContext {
    pub workflow_id: String,
    pub inputs: HashMap<String, serde_json::Value>,
    pub step_outputs: HashMap<String, HashMap<String, serde_json::Value>>,
    pub step_status: HashMap<String, StepStatus>,
    pub variables: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone)]
pub enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed { error: String, attempt: u32 },
    Skipped,
    Cancelled,
}

impl WorkflowExecutor {
    pub async fn execute_workflow(
        &self,
        definition: WorkflowDefinition,
        inputs: HashMap<String, serde_json::Value>,
    ) -> Result<WorkflowResult> {
        // Build execution DAG
        let dag = self.build_execution_dag(&definition)?;
        
        // Initialize execution context
        let context = ExecutionContext {
            workflow_id: Uuid::new_v4().to_string(),
            inputs,
            step_outputs: HashMap::new(),
            step_status: HashMap::new(),
            variables: HashMap::new(),
        };
        
        // Execute workflow
        self.execute_dag(dag, context).await
    }
    
    fn build_execution_dag(&self, definition: &WorkflowDefinition) -> Result<Graph<WorkflowStep, ()>> {
        let mut graph = Graph::new();
        let mut node_map = HashMap::new();
        
        // Add all steps as nodes
        for step in &definition.steps {
            let node_index = graph.add_node(step.clone());
            node_map.insert(step.id.clone(), node_index);
        }
        
        // Add dependency edges
        for step in &definition.steps {
            if let Some(dependencies) = &step.depends_on {
                for dep in dependencies {
                    if let (Some(&dep_node), Some(&step_node)) = 
                        (node_map.get(dep), node_map.get(&step.id)) {
                        graph.add_edge(dep_node, step_node, ());
                    }
                }
            }
        }
        
        // Validate DAG (no cycles)
        if petgraph::algo::is_cyclic_directed(&graph) {
            return Err(anyhow!("Workflow contains circular dependencies"));
        }
        
        Ok(graph)
    }
}
```

#### 2.2 Parallel Execution Framework
```rust
use tokio::sync::Semaphore;
use futures::stream::{FuturesUnordered, StreamExt};

pub struct ParallelExecutor {
    max_concurrent_steps: usize,
    semaphore: Arc<Semaphore>,
}

impl ParallelExecutor {
    pub async fn execute_parallel_steps(
        &self,
        steps: Vec<WorkflowStep>,
        context: Arc<RwLock<ExecutionContext>>,
        tool_registry: Arc<ToolRegistry>,
    ) -> Result<Vec<StepResult>> {
        let mut futures = FuturesUnordered::new();
        
        for step in steps {
            let permit = self.semaphore.clone().acquire_owned().await?;
            let context_clone = context.clone();
            let registry_clone = tool_registry.clone();
            
            futures.push(tokio::spawn(async move {
                let _permit = permit; // Hold permit for duration
                Self::execute_single_step(step, context_clone, registry_clone).await
            }));
        }
        
        let mut results = Vec::new();
        while let Some(result) = futures.next().await {
            results.push(result??);
        }
        
        Ok(results)
    }
}
```

### Phase 3: Template Engine and Variable Resolution (2-3 weeks)

#### 3.1 Template Processing System
```rust
use handlebars::Handlebars;
use serde_json::Value;

pub struct TemplateEngine {
    handlebars: Handlebars<'static>,
}

impl TemplateEngine {
    pub fn new() -> Self {
        let mut handlebars = Handlebars::new();
        
        // Register custom helpers
        handlebars.register_helper("json_path", Box::new(json_path_helper));
        handlebars.register_helper("base64_encode", Box::new(base64_encode_helper));
        handlebars.register_helper("regex_match", Box::new(regex_match_helper));
        
        Self { handlebars }
    }
    
    pub fn resolve_parameters(
        &self,
        parameters: &HashMap<String, Value>,
        context: &ExecutionContext,
    ) -> Result<HashMap<String, Value>> {
        let mut resolved = HashMap::new();
        
        for (key, value) in parameters {
            let resolved_value = self.resolve_value(value, context)?;
            resolved.insert(key.clone(), resolved_value);
        }
        
        Ok(resolved)
    }
    
    fn resolve_value(&self, value: &Value, context: &ExecutionContext) -> Result<Value> {
        match value {
            Value::String(s) => {
                if s.contains("{{") {
                    let template_data = self.build_template_data(context)?;
                    let rendered = self.handlebars.render_template(s, &template_data)?;
                    Ok(Value::String(rendered))
                } else {
                    Ok(value.clone())
                }
            }
            Value::Object(obj) => {
                let mut resolved_obj = serde_json::Map::new();
                for (k, v) in obj {
                    resolved_obj.insert(k.clone(), self.resolve_value(v, context)?);
                }
                Ok(Value::Object(resolved_obj))
            }
            Value::Array(arr) => {
                let resolved_arr: Result<Vec<_>> = arr.iter()
                    .map(|v| self.resolve_value(v, context))
                    .collect();
                Ok(Value::Array(resolved_arr?))
            }
            _ => Ok(value.clone()),
        }
    }
}
```

### Phase 4: Error Handling and Recovery (2-3 weeks)

#### 4.1 Comprehensive Error Handling
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorHandlingConfig {
    pub on_failure: FailureStrategy,
    pub compensation: Option<CompensationConfig>,
    pub notifications: Option<Vec<NotificationConfig>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum FailureStrategy {
    Fail,
    Continue,
    Retry { config: RetryConfig },
    Compensate { steps: Vec<String> },
    Rollback,
}

pub struct CompensationExecutor {
    tool_registry: Arc<ToolRegistry>,
}

impl CompensationExecutor {
    pub async fn execute_compensation(
        &self,
        failed_step: &str,
        completed_steps: &[String],
        context: &ExecutionContext,
    ) -> Result<()> {
        // Execute compensation logic for completed steps
        for step_id in completed_steps.iter().rev() {
            if let Some(compensation) = self.get_compensation_for_step(step_id) {
                self.execute_compensation_step(compensation, context).await?;
            }
        }
        Ok(())
    }
}
```

### Phase 5: Workflow Persistence and State Management (2-3 weeks)

#### 5.1 Workflow State Persistence
```rust
use sqlx::{SqlitePool, Row};

pub struct WorkflowStateManager {
    db_pool: SqlitePool,
}

impl WorkflowStateManager {
    pub async fn save_workflow_state(
        &self,
        workflow_id: &str,
        state: &WorkflowState,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT OR REPLACE INTO workflow_executions 
            (id, definition, state, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
            workflow_id,
            serde_json::to_string(&state.definition)?,
            serde_json::to_string(&state.execution_context)?,
            state.created_at,
            chrono::Utc::now()
        )
        .execute(&self.db_pool)
        .await?;
        
        Ok(())
    }
    
    pub async fn resume_workflow(&self, workflow_id: &str) -> Result<WorkflowState> {
        let row = sqlx::query!(
            "SELECT definition, state FROM workflow_executions WHERE id = ?",
            workflow_id
        )
        .fetch_one(&self.db_pool)
        .await?;
        
        let definition: WorkflowDefinition = serde_json::from_str(&row.definition)?;
        let context: ExecutionContext = serde_json::from_str(&row.state)?;
        
        Ok(WorkflowState {
            definition,
            execution_context: context,
            created_at: chrono::Utc::now(), // This should be loaded from DB
        })
    }
}
```

## Integration Points

### 1. CLI Command Extensions
```bash
# Execute workflow from file
fluent openai workflow run \
  --file workflows/code_analysis.yaml \
  --input project_path=/path/to/project \
  --input language=rust

# Execute workflow with MCP tools
fluent openai workflow run \
  --file workflows/deployment.yaml \
  --mcp-servers "k8s:kubectl-mcp,monitoring:prometheus-mcp" \
  --parallel-limit 5

# Resume failed workflow
fluent openai workflow resume \
  --workflow-id abc-123-def \
  --from-step analyze_code

# List workflow executions
fluent openai workflow list \
  --status failed \
  --since "2024-01-01"
```

### 2. Agent Integration
```rust
impl AgentWithMcp {
    pub async fn execute_workflow_goal(
        &self,
        goal: &str,
        workflow_template: Option<&str>,
    ) -> Result<GoalResult> {
        // Generate or load workflow definition
        let workflow = if let Some(template) = workflow_template {
            self.load_workflow_template(template).await?
        } else {
            self.generate_workflow_from_goal(goal).await?
        };
        
        // Execute workflow with agent oversight
        let executor = WorkflowExecutor::new(self.tool_registry.clone());
        let result = executor.execute_workflow(workflow, HashMap::new()).await?;
        
        // Learn from execution
        self.memory.store_workflow_execution(&result).await?;
        
        Ok(GoalResult::from_workflow_result(result))
    }
}
```

## Risk Assessment and Mitigation

### High-Risk Areas
1. **Workflow Complexity**: Complex DAGs with many dependencies
   - **Mitigation**: Workflow validation, complexity limits, visualization tools
2. **Resource Management**: Memory and CPU usage for large workflows
   - **Mitigation**: Resource limits, streaming execution, checkpointing
3. **Error Propagation**: Cascading failures in complex workflows
   - **Mitigation**: Circuit breakers, bulkheads, graceful degradation

### Medium-Risk Areas
1. **Template Security**: Code injection through templates
   - **Mitigation**: Sandboxed template execution, input validation
2. **State Consistency**: Concurrent workflow modifications
   - **Mitigation**: Optimistic locking, event sourcing

## Implementation Milestones

### Milestone 1: Workflow Definition (Week 1-2)
- [ ] YAML schema definition
- [ ] Workflow validation framework
- [ ] Basic template engine
- [ ] Unit tests for workflow parsing

### Milestone 2: Execution Engine (Week 3-5)
- [ ] DAG construction and validation
- [ ] Sequential execution engine
- [ ] Parallel execution framework
- [ ] Integration tests

### Milestone 3: Advanced Features (Week 6-8)
- [ ] Error handling and compensation
- [ ] Workflow persistence
- [ ] Resume and recovery mechanisms
- [ ] Performance optimization

### Milestone 4: Production Features (Week 9-11)
- [ ] Monitoring and metrics
- [ ] CLI integration
- [ ] Documentation and examples
- [ ] Load testing and optimization

## Success Metrics

### Technical Metrics
- **Execution Performance**: 1000+ step workflows in < 5 minutes
- **Parallel Efficiency**: 80%+ CPU utilization with parallel steps
- **Reliability**: 99.9% workflow completion rate
- **Recovery**: < 30 seconds to resume failed workflows

### Functional Metrics
- **Workflow Complexity**: Support 100+ step workflows
- **Template Flexibility**: Support complex data transformations
- **Error Handling**: Comprehensive failure recovery
- **Integration**: Seamless MCP tool integration

## Estimated Effort

**Total Effort**: 13-18 weeks
- **Development**: 10-14 weeks (2-3 senior developers)
- **Testing**: 2-3 weeks
- **Documentation**: 1 week

**Complexity**: High
- **Technical Complexity**: DAG algorithms, parallel execution, state management
- **Integration Complexity**: Tool registry, MCP integration, agent coordination
- **Testing Complexity**: Workflow simulation, failure injection, performance testing

This implementation will establish Fluent CLI as a comprehensive workflow orchestration platform for agentic AI systems.
