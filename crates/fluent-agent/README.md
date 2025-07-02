# Fluent Agent - Advanced Agentic AI Framework

The `fluent-agent` crate provides a sophisticated agentic AI framework that implements the ReAct (Reasoning, Acting, Observing) pattern for autonomous goal achievement. This framework enables AI agents to reason about problems, plan and execute actions, observe results, and learn from experiences.

## 🌟 Key Features

### 🧠 Advanced Reasoning Engine
- **LLM-powered reasoning** with sophisticated prompt engineering
- **Multiple reasoning types**: Goal analysis, task decomposition, action planning, context analysis, self-reflection
- **Confidence scoring** and goal achievement assessment
- **Pattern extraction** from reasoning outputs

### 🎯 Intelligent Action Planning
- **Multi-strategy planning** with risk assessment
- **Alternative action generation** for high-risk scenarios
- **Resource allocation** and timing optimization
- **Success criteria definition** and validation

### 👁️ Comprehensive Observation System
- **Action result analysis** with quality scoring
- **Pattern detection** in execution history
- **Impact assessment** of actions and changes
- **Learning extraction** from observations

### 🧠 Advanced Memory System
- **Short-term memory** for immediate context and working hypotheses
- **Long-term memory** for persistent learnings and strategies
- **Episodic memory** for specific experiences and outcomes
- **Semantic memory** for general knowledge and rules
- **Memory consolidation** and attention management

### 🎭 Goal-Oriented Execution
- **Flexible goal types**: Code generation, analysis, testing, debugging, etc.
- **Goal templates** for common tasks
- **Progress tracking** and success measurement
- **Timeout and iteration limits**

## 🏗️ Architecture

The framework follows a modular architecture with clear separation of concerns:

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Orchestrator  │────│ Reasoning Engine│────│ Action Planner  │
│                 │    │                 │    │                 │
│ - ReAct Loop    │    │ - Goal Analysis │    │ - Risk Assessment│
│ - Goal Tracking │    │ - Task Planning │    │ - Strategy Select│
│ - State Mgmt    │    │ - Self Reflect  │    │ - Alternative Gen│
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         │              ┌─────────────────┐              │
         │──────────────│ Action Executor │──────────────│
         │              │                 │              │
         │              │ - Tool Execution│              │
         │              │ - Code Generation│             │
         │              │ - File Operations│             │
         │              └─────────────────┘              │
         │                       │                       │
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│ Memory System   │────│ Observation     │────│ Context Manager │
│                 │    │ Processor       │    │                 │
│ - Short/Long    │    │                 │    │ - Execution     │
│ - Episodic      │    │ - Result Analysis│    │ - Variables     │
│ - Semantic      │    │ - Pattern Detect │    │ - History       │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

## 🚀 Quick Start

### Basic Usage

```rust
use fluent_agent::{
    AgentOrchestrator, Goal, GoalType, GoalTemplates,
    LLMReasoningEngine, IntelligentActionPlanner, 
    ComprehensiveActionExecutor, ComprehensiveObservationProcessor,
    MemorySystem, MemoryConfig,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create engine (OpenAI, Claude, etc.)
    let engine = create_your_engine().await?;
    
    // Set up agent components
    let reasoning_engine = Arc::new(LLMReasoningEngine::new(engine));
    let action_planner = Arc::new(IntelligentActionPlanner::new(risk_assessor));
    let action_executor = Arc::new(ComprehensiveActionExecutor::new(
        tool_executor, code_generator, file_manager
    ));
    let observation_processor = Arc::new(ComprehensiveObservationProcessor::new(
        result_analyzer, pattern_detector, impact_assessor, learning_extractor
    ));
    let memory_system = Arc::new(MemorySystem::new(
        long_term_memory, episodic_memory, semantic_memory, MemoryConfig::default()
    ));
    
    // Create agent orchestrator
    let mut agent = AgentOrchestrator::new(
        reasoning_engine,
        action_planner,
        action_executor,
        observation_processor,
        memory_system,
    );
    
    // Create a goal
    let goal = GoalTemplates::code_generation(
        "Create a REST API server in Rust".to_string(),
        "Rust".to_string(),
        vec![
            "Use Axum framework".to_string(),
            "Include error handling".to_string(),
            "Add comprehensive tests".to_string(),
        ],
    );
    
    // Execute the goal
    let result = agent.execute_goal(goal).await?;
    
    println!("Success: {}", result.success);
    println!("Final output: {:?}", result.final_output);
    
    Ok(())
}
```

### Goal Templates

The framework provides convenient goal templates for common tasks:

```rust
// Code generation
let goal = GoalTemplates::code_generation(
    "Create a binary search function".to_string(),
    "Rust".to_string(),
    vec!["Generic implementation".to_string(), "Include tests".to_string()],
);

// Code review
let goal = GoalTemplates::code_review(
    "src/main.rs".to_string(),
    vec!["Performance".to_string(), "Security".to_string()],
);

// Debugging
let goal = GoalTemplates::debugging(
    "Memory leak in server".to_string(),
    "Process memory grows continuously".to_string(),
);

// Testing
let goal = GoalTemplates::testing(
    "user authentication module".to_string(),
    vec!["Unit tests".to_string(), "Integration tests".to_string()],
);
```

## 🔧 Customization

### Custom Reasoning Strategies

```rust
use fluent_agent::{ReasoningEngine, ReasoningResult, ExecutionContext};

struct CustomReasoningEngine {
    // Your implementation
}

#[async_trait]
impl ReasoningEngine for CustomReasoningEngine {
    async fn reason(&self, context: &ExecutionContext) -> Result<ReasoningResult> {
        // Your custom reasoning logic
    }
    
    fn get_capabilities(&self) -> Vec<ReasoningCapability> {
        // Define your capabilities
    }
    
    fn can_handle(&self, reasoning_type: &ReasoningType) -> bool {
        // Define what reasoning types you support
    }
}
```

### Custom Action Executors

```rust
use fluent_agent::{ActionExecutor, ActionPlan, ActionResult, ExecutionContext};

struct CustomActionExecutor {
    // Your implementation
}

#[async_trait]
impl ActionExecutor for CustomActionExecutor {
    async fn execute(&self, plan: ActionPlan, context: &mut ExecutionContext) -> Result<ActionResult> {
        // Your custom action execution logic
    }
    
    fn get_capabilities(&self) -> Vec<ExecutionCapability> {
        // Define your capabilities
    }
    
    fn can_execute(&self, action_type: &ActionType) -> bool {
        // Define what action types you support
    }
}
```

## 📊 Monitoring and Metrics

The framework provides comprehensive metrics and monitoring:

```rust
// Get execution metrics
let metrics = agent.get_metrics().await;
println!("Total iterations: {}", metrics.total_iterations);
println!("Success rate: {:.2}%", metrics.success_rate());
println!("Average reasoning time: {:?}", metrics.average_reasoning_time);

// Get memory statistics
let memory_stats = agent.get_memory_stats().await;
println!("Short-term items: {}", memory_stats.short_term_items);
println!("Active patterns: {}", memory_stats.active_patterns);
```

## 🎯 Use Cases

- **Automated Code Generation**: Create complete applications, functions, and modules
- **Code Review and Analysis**: Comprehensive code quality assessment
- **Debugging and Problem Solving**: Systematic issue identification and resolution
- **Test Generation**: Automated test suite creation and validation
- **Documentation**: Generate comprehensive documentation from code
- **Refactoring**: Intelligent code restructuring and optimization
- **Research and Learning**: Autonomous information gathering and synthesis

## 🔮 Advanced Features

### Memory Consolidation
The memory system automatically consolidates important short-term memories into long-term storage based on relevance and importance scores.

### Pattern Recognition
The observation processor detects patterns in execution history, enabling the agent to learn from past experiences and improve future performance.

### Self-Reflection
The reasoning engine includes self-reflection capabilities, allowing the agent to analyze its own performance and adjust strategies accordingly.

### Risk Assessment
The action planner includes sophisticated risk assessment, generating alternative actions for high-risk scenarios.

## 📚 Examples

See the `examples/` directory for comprehensive examples:

- `agentic_example.rs` - Complete agent setup and execution
- `custom_reasoning.rs` - Custom reasoning engine implementation
- `memory_usage.rs` - Advanced memory system usage
- `goal_templates.rs` - Using and creating goal templates

## 🤝 Contributing

Contributions are welcome! Please see the main project's contributing guidelines.

## 📄 License

This project is licensed under the same terms as the main fluent_cli project.
