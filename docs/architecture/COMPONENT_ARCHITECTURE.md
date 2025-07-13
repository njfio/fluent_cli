# Component Architecture Documentation

## Overview

This document provides detailed architectural information for each major component in the Fluent CLI system, including their responsibilities, interfaces, dependencies, and internal structure.

## Component Hierarchy

```
┌─────────────────────────────────────────────────────────────────┐
│                         fluent-cli                              │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
│  │   Commands      │  │    Agentic      │  │    Memory       │  │
│  │   Module        │  │   Orchestrator  │  │   Manager       │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                                │
┌─────────────────────────────────────────────────────────────────┐
│                        fluent-agent                             │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
│  │  Orchestrator   │  │     Tools       │  │      MCP        │  │
│  │     Engine      │  │   Registry      │  │    Adapter      │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                                │
┌─────────────────────────────────────────────────────────────────┐
│                        fluent-core                              │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
│  │     Types       │  │     Traits      │  │     Config      │  │
│  │   & Errors      │  │  & Interfaces   │  │   Management    │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                                │
┌─────────────────────────────────────────────────────────────────┐
│                       fluent-engines                            │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
│  │   LLM Engine    │  │    Pipeline     │  │   Connection    │  │
│  │ Implementations │  │    Executor     │  │     Pool        │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

## Core Components

### 1. Command System (`fluent-cli/commands`)

#### Architecture
```rust
pub trait CommandHandler {
    async fn execute(&self, matches: &ArgMatches, config: &Config) -> Result<()>;
}
```

#### Components
- **AgentCommand**: Handles agent-related operations
- **EngineCommand**: Direct LLM engine interactions
- **McpCommand**: MCP server management
- **Neo4jCommand**: Graph database operations
- **PipelineCommand**: Pipeline execution

#### Responsibilities
- Parse command-line arguments
- Validate input parameters
- Route to appropriate handlers
- Format and display results
- Handle errors gracefully

#### Dependencies
- `fluent-core` for configuration and types
- `fluent-engines` for LLM interactions
- `fluent-agent` for agentic capabilities

### 2. Agentic Orchestrator (`fluent-agent/orchestrator`)

#### Architecture
```rust
pub struct AgentOrchestrator {
    reasoning_engine: Box<dyn ReasoningEngine>,
    action_planner: Box<dyn ActionPlanner>,
    action_executor: Box<dyn ActionExecutor>,
    observation_processor: Box<dyn ObservationProcessor>,
    memory_system: Arc<dyn MemorySystem>,
}
```

#### ReAct Loop Implementation
```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│  Reasoning  │───▶│   Action    │───▶│  Execution  │───▶│ Observation │
│   Engine    │    │  Planning   │    │   Engine    │    │ Processing  │
└─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
       ▲                                                        │
       │                                                        │
       └────────────────────────────────────────────────────────┘
                            Memory Update
```

#### Key Features
- **Goal-Oriented Execution**: Autonomous goal achievement
- **Context Management**: Maintains execution context
- **Error Recovery**: Handles failures gracefully
- **Performance Monitoring**: Tracks execution metrics

### 3. Engine Abstraction (`fluent-core/traits`)

#### Core Engine Trait
```rust
#[async_trait]
pub trait Engine: Send + Sync {
    async fn execute(&self, request: &Request) -> Result<Response>;
    async fn upsert(&self, request: &UpsertRequest) -> Result<UpsertResponse>;
    fn get_neo4j_client(&self) -> Option<&Arc<Neo4jClient>>;
    fn get_session_id(&self) -> Option<String>;
    fn extract_content(&self, value: &Value) -> Option<ExtractedContent>;
    async fn upload_file(&self, file_path: &Path) -> Result<String>;
}
```

#### Engine Implementations
- **OpenAI Engine**: GPT models integration
- **Anthropic Engine**: Claude models integration
- **Google Gemini Engine**: Gemini models integration
- **Mistral Engine**: Mistral AI integration
- **Cohere Engine**: Cohere models integration
- **15+ Additional Providers**: Comprehensive LLM support

### 4. Pipeline Execution System (`fluent-engines/pipeline`)

#### Architecture
```rust
pub struct ModularPipelineExecutor {
    step_executor: StepExecutor,
    parallel_executor: ParallelExecutor,
    condition_executor: ConditionExecutor,
    loop_executor: LoopExecutor,
    variable_expander: VariableExpander,
}
```

#### Pipeline Step Types
- **LLM Steps**: Direct LLM interactions
- **Parallel Steps**: Concurrent execution
- **Conditional Steps**: Logic-based branching
- **Loop Steps**: Iterative processing
- **Command Steps**: System command execution

#### Features
- **YAML Configuration**: Human-readable pipeline definitions
- **Variable Substitution**: Dynamic content injection
- **Error Handling**: Comprehensive error management
- **State Management**: Persistent execution state

### 5. Memory System (`fluent-agent/memory`)

#### Architecture
```rust
pub struct SqliteMemoryStore {
    connection: Arc<Mutex<Connection>>,
    config: MemoryConfig,
}

pub trait MemorySystem: Send + Sync {
    async fn store_memory(&self, memory: &Memory) -> Result<String>;
    async fn retrieve_memories(&self, query: &str, limit: usize) -> Result<Vec<Memory>>;
    async fn update_memory(&self, id: &str, memory: &Memory) -> Result<()>;
    async fn delete_memory(&self, id: &str) -> Result<()>;
}
```

#### Memory Types
- **Conversation Memory**: Chat history and context
- **Task Memory**: Goal and task-related information
- **Learning Memory**: Insights and patterns
- **System Memory**: Configuration and state

#### Features
- **SQLite Backend**: Reliable local storage
- **Importance Scoring**: Relevance-based retrieval
- **Memory Consolidation**: Automatic cleanup
- **Query Interface**: Flexible memory search

### 6. Tool Registry (`fluent-agent/tools`)

#### Architecture
```rust
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
    capabilities: HashSet<ToolCapability>,
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> &[ToolParameter];
    async fn execute(&self, params: &ToolParameters) -> Result<ToolResult>;
}
```

#### Built-in Tools
- **File Operations**: Read, write, search files
- **Code Analysis**: AST parsing, symbol extraction
- **System Commands**: Shell command execution
- **Web Operations**: HTTP requests, web scraping
- **Database Operations**: SQL queries, data manipulation

### 7. MCP Integration (`fluent-agent/mcp_adapter`)

#### Architecture
```rust
pub struct FluentMcpServer {
    tool_registry: Arc<ToolRegistry>,
    memory_system: Arc<dyn MemorySystem>,
    transport: McpTransport,
}
```

#### MCP Protocol Support
- **JSON-RPC 2.0**: Standard protocol implementation
- **Tool Exposure**: All tools available via MCP
- **Memory Access**: Persistent memory via MCP
- **STDIO Transport**: Command-line integration
- **HTTP Transport**: Network-based communication (planned)

## Data Flow Patterns

### 1. Request Processing Flow
```
CLI Input → Validation → Command Router → Handler → Engine → Provider API → Response
```

### 2. Agentic Execution Flow
```
Goal → Reasoning → Planning → Action → Execution → Observation → Memory → [Loop]
```

### 3. Pipeline Execution Flow
```
YAML → Parser → Step Executor → Engine Calls → Result Aggregation → Output
```

### 4. Memory Management Flow
```
Event → Memory Creation → Importance Scoring → Storage → Retrieval → Context Integration
```

## Inter-Component Communication

### 1. Synchronous Communication
- Direct function calls within the same process
- Trait-based interfaces for abstraction
- Result types for error handling

### 2. Asynchronous Communication
- Async/await for I/O operations
- Channel-based communication for concurrent operations
- Event-driven architecture for loose coupling

### 3. External Communication
- HTTP/HTTPS for API calls
- JSON-RPC for MCP protocol
- SQLite for data persistence
- File system for configuration and logs

## Configuration Management

### 1. Configuration Hierarchy
```
Default Config → System Config → User Config → Environment Variables → CLI Arguments
```

### 2. Configuration Sources
- **YAML Files**: Primary configuration format
- **Environment Variables**: Runtime overrides
- **Command Line**: Session-specific settings
- **Runtime Updates**: Dynamic configuration changes

### 3. Configuration Validation
- Schema validation for YAML files
- Type checking for all parameters
- Security validation for sensitive data
- Compatibility checking for version updates

## Error Handling Strategy

### 1. Error Types
- **Validation Errors**: Input and configuration validation
- **Network Errors**: API communication failures
- **System Errors**: File system and resource errors
- **Logic Errors**: Business logic failures

### 2. Error Propagation
- Result types throughout the system
- Structured error information
- Context preservation
- Graceful degradation

### 3. Recovery Mechanisms
- Automatic retry for transient failures
- Fallback strategies for service unavailability
- State recovery for interrupted operations
- User notification for manual intervention

This component architecture provides a comprehensive view of how the Fluent CLI system is structured and how its components interact to deliver powerful LLM and agentic capabilities.
