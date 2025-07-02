# Model Context Protocol (MCP) Integration Plan for Fluent CLI

## 🎯 Overview

This document outlines the comprehensive plan to integrate Model Context Protocol (MCP) support into the fluent_cli agentic system, enabling standardized tool communication and interoperability with other AI systems.

## 📋 MCP Background

### What is MCP?
Model Context Protocol (MCP) is an open standard developed by Anthropic that enables seamless integration between LLM applications and external data sources and tools. It uses JSON-RPC 2.0 as the underlying communication protocol.

### Key MCP Concepts
- **Tools**: Executable functions that can be called by the LLM
- **Resources**: Data sources that can be read by the LLM
- **Prompts**: Template prompts that can be used by the LLM
- **Servers**: Applications that expose tools, resources, and prompts
- **Clients**: Applications that consume MCP services

## 🏗️ Architecture Design

### Current State Analysis
The fluent_cli agentic system currently has:
- Tool registry with custom tool execution interface
- Engine abstraction for LLM communication
- Agent orchestrator for autonomous operation
- Memory system for context management

### Target MCP Architecture
```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   MCP Client    │    │  Fluent Agent   │    │   MCP Server    │
│                 │    │   Orchestrator  │    │                 │
│ ┌─────────────┐ │    │ ┌─────────────┐ │    │ ┌─────────────┐ │
│ │ External    │ │◄──►│ │ Tool        │ │◄──►│ │ Tool        │ │
│ │ MCP Servers │ │    │ │ Registry    │ │    │ │ Exposer     │ │
│ └─────────────┘ │    │ └─────────────┘ │    │ └─────────────┘ │
│                 │    │                 │    │                 │
│ ┌─────────────┐ │    │ ┌─────────────┐ │    │ ┌─────────────┐ │
│ │ Resource    │ │    │ │ Memory      │ │    │ │ Resource    │ │
│ │ Consumer    │ │    │ │ System      │ │    │ │ Provider    │ │
│ └─────────────┘ │    │ └─────────────┘ │    │ └─────────────┘ │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         └───────────────────────┼───────────────────────┘
                                 │
                    ┌─────────────────┐
                    │ JSON-RPC 2.0    │
                    │ Transport Layer │
                    │ (stdio/HTTP/WS) │
                    └─────────────────┘
```

## 🔧 Implementation Plan

### Phase 1: Core MCP Infrastructure (Week 1-2)

#### 1.1 MCP Transport Layer
**File**: `crates/fluent-mcp/src/transport/mod.rs`
```rust
pub trait McpTransport: Send + Sync {
    async fn send(&self, message: JsonRpcMessage) -> Result<()>;
    async fn receive(&self) -> Result<JsonRpcMessage>;
    async fn close(&self) -> Result<()>;
}

pub struct StdioTransport;
pub struct HttpTransport;
pub struct WebSocketTransport;
```

#### 1.2 JSON-RPC Message Types
**File**: `crates/fluent-mcp/src/messages/mod.rs`
```rust
#[derive(Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub result: Option<Value>,
    pub error: Option<JsonRpcError>,
}
```

#### 1.3 MCP Protocol Messages
**File**: `crates/fluent-mcp/src/protocol/mod.rs`
```rust
// Initialize protocol
pub struct InitializeRequest {
    pub protocol_version: String,
    pub capabilities: ClientCapabilities,
    pub client_info: ClientInfo,
}

// Tool-related messages
pub struct ListToolsRequest;
pub struct CallToolRequest {
    pub name: String,
    pub arguments: Option<Value>,
}

// Resource-related messages
pub struct ListResourcesRequest;
pub struct ReadResourceRequest {
    pub uri: String,
}
```

### Phase 2: MCP Server Implementation (Week 3)

#### 2.1 MCP Server Core
**File**: `crates/fluent-mcp/src/server/mod.rs`
```rust
pub struct McpServer {
    transport: Box<dyn McpTransport>,
    tool_registry: Arc<ToolRegistry>,
    resource_provider: Arc<dyn ResourceProvider>,
    prompt_provider: Arc<dyn PromptProvider>,
}

impl McpServer {
    pub async fn start(&self) -> Result<()>;
    pub async fn handle_request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse>;
    pub async fn shutdown(&self) -> Result<()>;
}
```

#### 2.2 Tool Exposure
**File**: `crates/fluent-mcp/src/server/tools.rs`
```rust
pub struct McpToolAdapter {
    tool_registry: Arc<ToolRegistry>,
}

impl McpToolAdapter {
    pub async fn list_tools(&self) -> Result<Vec<McpTool>>;
    pub async fn call_tool(&self, name: &str, args: Value) -> Result<McpToolResult>;
}
```

#### 2.3 Resource Provider
**File**: `crates/fluent-mcp/src/server/resources.rs`
```rust
pub trait ResourceProvider: Send + Sync {
    async fn list_resources(&self) -> Result<Vec<McpResource>>;
    async fn read_resource(&self, uri: &str) -> Result<McpResourceContent>;
}

pub struct FluentResourceProvider {
    memory_system: Arc<MemorySystem>,
    file_system: Arc<FileSystemExecutor>,
}
```

### Phase 3: MCP Client Implementation (Week 4)

#### 3.1 MCP Client Core
**File**: `crates/fluent-mcp/src/client/mod.rs`
```rust
pub struct McpClient {
    transport: Box<dyn McpTransport>,
    server_capabilities: Option<ServerCapabilities>,
}

impl McpClient {
    pub async fn connect(&mut self) -> Result<()>;
    pub async fn list_tools(&self) -> Result<Vec<McpTool>>;
    pub async fn call_tool(&self, name: &str, args: Value) -> Result<McpToolResult>;
    pub async fn list_resources(&self) -> Result<Vec<McpResource>>;
    pub async fn read_resource(&self, uri: &str) -> Result<McpResourceContent>;
}
```

#### 3.2 External Server Integration
**File**: `crates/fluent-mcp/src/client/integration.rs`
```rust
pub struct ExternalMcpIntegration {
    clients: HashMap<String, McpClient>,
    tool_registry: Arc<ToolRegistry>,
}

impl ExternalMcpIntegration {
    pub async fn register_external_server(&mut self, name: String, endpoint: String) -> Result<()>;
    pub async fn discover_tools(&self) -> Result<Vec<ExternalTool>>;
    pub async fn execute_external_tool(&self, server: &str, tool: &str, args: Value) -> Result<String>;
}
```

### Phase 4: Integration with Fluent Agent (Week 5)

#### 4.1 Agent Orchestrator Integration
**File**: `crates/fluent-agent/src/mcp_integration.rs`
```rust
pub struct McpIntegratedOrchestrator {
    base_orchestrator: AgentOrchestrator,
    mcp_server: Option<McpServer>,
    mcp_clients: HashMap<String, McpClient>,
}

impl McpIntegratedOrchestrator {
    pub async fn start_mcp_server(&mut self, transport: Box<dyn McpTransport>) -> Result<()>;
    pub async fn connect_to_mcp_server(&mut self, name: String, endpoint: String) -> Result<()>;
    pub async fn execute_with_mcp_tools(&self, context: &ExecutionContext) -> Result<ActionResult>;
}
```

#### 4.2 Tool Registry Enhancement
**File**: `crates/fluent-agent/src/tools/mcp_tools.rs`
```rust
pub struct McpToolExecutor {
    mcp_clients: Arc<HashMap<String, McpClient>>,
}

#[async_trait]
impl ToolExecutor for McpToolExecutor {
    async fn execute_tool(&self, tool_name: &str, parameters: &HashMap<String, Value>) -> Result<String>;
    fn get_available_tools(&self) -> Vec<String>;
    fn get_tool_description(&self, tool_name: &str) -> Option<String>;
}
```

## 🧪 Testing Strategy

### Unit Tests
- JSON-RPC message serialization/deserialization
- Transport layer functionality
- Protocol compliance
- Tool execution through MCP

### Integration Tests
- End-to-end MCP server/client communication
- External MCP server integration
- Tool discovery and execution
- Resource access and management

### Compliance Tests
- MCP specification compliance
- Interoperability with other MCP implementations
- Error handling and edge cases
- Performance under load

## 📊 Success Metrics

### Functional Metrics
- ✅ Full MCP protocol compliance
- ✅ Support for all transport types (stdio, HTTP, WebSocket)
- ✅ Tool, resource, and prompt exposure
- ✅ External MCP server integration
- ✅ Backward compatibility with existing tools

### Performance Metrics
- 📈 <100ms latency for tool calls
- 📈 Support for 100+ concurrent MCP connections
- 📈 <1MB memory overhead per MCP client
- 📈 99.9% uptime for MCP server

### Quality Metrics
- 🧪 >95% test coverage
- 🧪 Zero security vulnerabilities
- 🧪 Full documentation coverage
- 🧪 Interoperability with 3+ other MCP implementations

## 🔄 Migration Strategy

### Backward Compatibility
- Existing tool registry remains functional
- Gradual migration of tools to MCP interface
- Dual-mode operation during transition
- Configuration-driven MCP enablement

### Rollout Plan
1. **Alpha**: Internal testing with basic MCP server
2. **Beta**: Limited external testing with select MCP clients
3. **RC**: Full feature testing and performance validation
4. **GA**: Production release with full MCP support

## 📚 Dependencies

### New Crate Dependencies
```toml
[dependencies]
serde_json = "1.0"
tokio-tungstenite = "0.20"  # WebSocket support
hyper = "0.14"              # HTTP transport
uuid = "1.0"                # Request IDs
tracing = "0.1"             # Structured logging
```

### Development Dependencies
```toml
[dev-dependencies]
tokio-test = "0.4"
mockall = "0.11"
wiremock = "0.5"
```

## 🎯 Next Steps

1. **Start MCP Specification Research** (Current)
2. **Set up fluent-mcp crate structure**
3. **Implement JSON-RPC transport layer**
4. **Create MCP protocol message types**
5. **Build MCP server implementation**
6. **Add MCP client capabilities**
7. **Integrate with existing agent system**
8. **Create comprehensive test suite**

---

*This plan provides a roadmap for full MCP integration while maintaining backward compatibility and ensuring high quality implementation.*
