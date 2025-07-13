# Data Flow Architecture

## Overview

This document describes the data flow patterns, message formats, and communication protocols used throughout the Fluent CLI system. Understanding these flows is crucial for system integration, debugging, and extending functionality.

## Core Data Types

### 1. Request/Response Cycle
```rust
// Core request structure
pub struct Request {
    pub flowname: String,    // Operation type identifier
    pub payload: String,     // Content to be processed
}

// Core response structure
pub struct Response {
    pub content: String,           // Generated content
    pub usage: Usage,             // Token usage statistics
    pub model: String,            // Model used for generation
    pub finish_reason: Option<String>, // Completion reason
    pub cost: Cost,               // Cost breakdown
}

// Usage tracking
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

// Cost tracking
pub struct Cost {
    pub prompt_cost: f64,      // USD cost for prompt tokens
    pub completion_cost: f64,  // USD cost for completion tokens
    pub total_cost: f64,       // Total USD cost
}
```

## Primary Data Flows

### 1. Direct LLM Interaction Flow

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│ User Input  │───▶│ CLI Parser  │───▶│ Validation  │───▶│ Engine      │
└─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
                                                                │
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│ User Output │◀───│ Formatter   │◀───│ Response    │◀───│ Provider    │
└─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
```

**Data Transformation Steps**:
1. **Input**: Raw user command string
2. **Parsing**: Extract engine name, flags, and content
3. **Validation**: Sanitize input, validate parameters
4. **Request Creation**: Build structured Request object
5. **Engine Selection**: Route to appropriate LLM provider
6. **API Call**: HTTP request to provider API
7. **Response Processing**: Parse provider response
8. **Cost Calculation**: Compute usage and cost metrics
9. **Formatting**: Apply output formatting
10. **Display**: Present results to user

### 2. Pipeline Execution Flow

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│ YAML File   │───▶│ Parser      │───▶│ Validator   │───▶│ Executor    │
└─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
                                                                │
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│ Results     │◀───│ Aggregator  │◀───│ Step        │◀───│ Engine      │
└─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
```

**Pipeline Data Structure**:
```yaml
# Example pipeline YAML
name: "content_generation"
description: "Multi-step content generation pipeline"
variables:
  topic: "AI architecture"
  style: "technical"

steps:
  - name: "research"
    type: "llm"
    engine: "openai"
    prompt: "Research {{topic}} and provide key points"
    
  - name: "outline"
    type: "llm"
    engine: "anthropic"
    prompt: "Create an outline for {{topic}} using: {{research.content}}"
    
  - name: "content"
    type: "parallel"
    steps:
      - name: "introduction"
        type: "llm"
        engine: "gemini"
        prompt: "Write introduction for {{topic}}"
      - name: "conclusion"
        type: "llm"
        engine: "mistral"
        prompt: "Write conclusion for {{topic}}"
```

### 3. Agentic Execution Flow (ReAct Pattern)

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│ Goal Input  │───▶│ Reasoning   │───▶│ Action      │───▶│ Tool        │
└─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
       ▲                                                        │
       │           ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
       └───────────│ Memory      │◀───│ Observation │◀───│ Execution   │
                   └─────────────┘    └─────────────┘    └─────────────┘
```

**Agentic Data Structures**:
```rust
// Goal definition
pub struct Goal {
    pub id: String,
    pub description: String,
    pub goal_type: GoalType,
    pub priority: GoalPriority,
    pub requirements: Vec<String>,
    pub success_criteria: Vec<String>,
    pub context: HashMap<String, String>,
}

// Action plan
pub struct ActionPlan {
    pub goal_id: String,
    pub actions: Vec<PlannedAction>,
    pub risk_assessment: RiskLevel,
    pub estimated_duration: Duration,
    pub required_tools: Vec<String>,
}

// Observation result
pub struct ObservationResult {
    pub action_id: String,
    pub success: bool,
    pub output: String,
    pub quality_score: f64,
    pub lessons_learned: Vec<String>,
    pub next_actions: Vec<String>,
}
```

### 4. MCP Integration Flow

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│ MCP Client  │───▶│ JSON-RPC    │───▶│ MCP Server  │───▶│ Tool        │
└─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
       ▲                                                        │
       │           ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
       └───────────│ Response    │◀───│ Formatter   │◀───│ Execution   │
                   └─────────────┘    └─────────────┘    └─────────────┘
```

**MCP Message Format**:
```json
// Tool list request
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/list",
  "params": {}
}

// Tool execution request
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/call",
  "params": {
    "name": "file_read",
    "arguments": {
      "path": "/path/to/file.txt"
    }
  }
}

// Response format
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "File content here..."
      }
    ]
  }
}
```

## Memory System Data Flow

### 1. Memory Storage Flow
```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│ Event       │───▶│ Memory      │───▶│ Importance  │───▶│ SQLite      │
│ Trigger     │    │ Creation    │    │ Scoring     │    │ Storage     │
└─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
```

### 2. Memory Retrieval Flow
```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│ Query       │───▶│ Search      │───▶│ Relevance   │───▶│ Context     │
│ Request     │    │ Engine      │    │ Ranking     │    │ Integration │
└─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
```

**Memory Data Structure**:
```rust
pub struct Memory {
    pub id: String,
    pub content: String,
    pub memory_type: MemoryType,
    pub importance: f64,
    pub created_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    pub access_count: u32,
    pub tags: Vec<String>,
    pub metadata: HashMap<String, String>,
}
```

## Configuration Data Flow

### 1. Configuration Loading Hierarchy
```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│ Default     │───▶│ System      │───▶│ User        │───▶│ Environment │
│ Config      │    │ Config      │    │ Config      │    │ Variables   │
└─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
                                                                │
                   ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
                   │ Final       │◀───│ Validation  │◀───│ CLI Args    │
                   │ Config      │    │ & Merging   │    │ Override    │
                   └─────────────┘    └─────────────┘    └─────────────┘
```

### 2. Configuration Structure
```rust
pub struct Config {
    pub engines: HashMap<String, EngineConfig>,
    pub neo4j: Option<Neo4jConfig>,
    pub cache: CacheConfig,
    pub security: SecurityConfig,
    pub logging: LoggingConfig,
    pub agent: AgentConfig,
}

pub struct EngineConfig {
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: String,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f64>,
    pub timeout: Duration,
}
```

## Error Handling Data Flow

### 1. Error Propagation Pattern
```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│ Error       │───▶│ Context     │───▶│ Recovery    │───▶│ User        │
│ Origin      │    │ Addition    │    │ Attempt     │    │ Notification│
└─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
```

### 2. Error Data Structure
```rust
#[derive(Debug, thiserror::Error)]
pub enum FluentError {
    #[error("Configuration error: {message}")]
    Configuration { message: String },
    
    #[error("Engine error: {engine} - {message}")]
    Engine { engine: String, message: String },
    
    #[error("Network error: {message}")]
    Network { message: String },
    
    #[error("Validation error: {field} - {message}")]
    Validation { field: String, message: String },
}
```

## Performance Monitoring Data Flow

### 1. Metrics Collection
```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│ Operation   │───▶│ Timing      │───▶│ Metrics     │───▶│ Storage/    │
│ Execution   │    │ Collection  │    │ Aggregation │    │ Reporting   │
└─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
```

### 2. Metrics Data Structure
```rust
pub struct PerformanceMetrics {
    pub operation: String,
    pub duration: Duration,
    pub tokens_used: u32,
    pub cost: f64,
    pub success: bool,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}
```

## Security Data Flow

### 1. Input Validation Flow
```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│ Raw Input   │───▶│ Sanitization│───▶│ Validation  │───▶│ Safe        │
└─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
```

### 2. Authentication Flow
```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│ API Key     │───▶│ Encryption  │───▶│ Storage     │───▶│ Runtime     │
│ Input       │    │ & Hashing   │    │ (Secure)    │    │ Usage       │
└─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
```

This data flow architecture provides a comprehensive understanding of how information moves through the Fluent CLI system, enabling effective debugging, monitoring, and system extension.
