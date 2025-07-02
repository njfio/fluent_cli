# MCP HTTP Transport Implementation Plan

## Executive Summary

This document outlines the implementation plan for adding HTTP transport support to Fluent CLI's Model Context Protocol (MCP) implementation, currently limited to STDIO transport. The plan includes WebSocket support for real-time communication, resource subscriptions, and comprehensive HTTP-based MCP capabilities.

## Current State Analysis

### Existing Implementation
- **File**: `crates/fluent-agent/src/mcp_client.rs`
- **Transport**: STDIO only (process-based communication)
- **Protocol**: JSON-RPC 2.0 over STDIO pipes
- **Limitations**: 
  - No network-based communication
  - Limited to local process spawning
  - No real-time subscriptions
  - Single connection per server process

### Architecture Assessment
```rust
// Current STDIO-based approach
pub struct McpClient {
    process: Option<Child>,
    stdin: Option<Arc<Mutex<ChildStdin>>>,
    stdout: Option<Arc<Mutex<BufReader<ChildStdout>>>>,
    response_handlers: Arc<RwLock<HashMap<String, mpsc::UnboundedSender<JsonRpcResponse>>>>,
    tools: Arc<RwLock<Vec<McpTool>>>,
    resources: Arc<RwLock<Vec<McpResource>>>,
    capabilities: Option<ServerCapabilities>,
}
```

## Technical Research Summary

### HTTP vs STDIO Transport Patterns
1. **STDIO Advantages**: Low latency, process isolation, simple setup
2. **HTTP Advantages**: Network accessibility, scalability, standard tooling
3. **Industry Standards**: Most RPC protocols support both (gRPC, JSON-RPC)

### Communication Patterns Analysis
1. **HTTP Polling**: Simple, reliable, higher latency
2. **WebSockets**: Real-time, bidirectional, connection management complexity
3. **Server-Sent Events**: Unidirectional real-time, simpler than WebSockets

### Rust HTTP Ecosystem
- **Client**: `reqwest` (high-level), `hyper` (low-level)
- **Server**: `axum`, `warp`, `actix-web`
- **WebSocket**: `tokio-tungstenite`, `axum` WebSocket support
- **HTTP/2**: Native support in `hyper` and `reqwest`

## Implementation Plan

### Phase 1: HTTP Client Transport (4-6 weeks)

#### 1.1 Transport Abstraction Layer
```rust
#[async_trait]
pub trait McpTransport: Send + Sync {
    async fn send_request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse>;
    async fn start_listening(&self) -> Result<mpsc::Receiver<JsonRpcNotification>>;
    async fn close(&self) -> Result<()>;
}

pub struct StdioTransport {
    // Existing implementation
}

pub struct HttpTransport {
    client: reqwest::Client,
    base_url: String,
    auth_token: Option<String>,
}

pub struct WebSocketTransport {
    ws_stream: Option<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    message_tx: mpsc::UnboundedSender<Message>,
    response_rx: mpsc::UnboundedReceiver<JsonRpcResponse>,
}
```

#### 1.2 HTTP Transport Implementation
**Dependencies to add:**
```toml
[dependencies]
reqwest = { version = "0.11", features = ["json", "stream"] }
tokio-tungstenite = "0.20"
url = "2.4"
```

**Key Components:**
1. **HTTP Client**: RESTful JSON-RPC over HTTP POST
2. **Connection Pooling**: Reuse connections for performance
3. **Authentication**: Bearer token, API key support
4. **Retry Logic**: Exponential backoff for failed requests
5. **Timeout Management**: Configurable request timeouts

#### 1.3 Configuration Enhancement
```yaml
mcp:
  servers:
    filesystem:
      transport: "stdio"
      command: "mcp-server-filesystem"
      args: ["--stdio"]
    
    remote-tools:
      transport: "http"
      url: "https://api.example.com/mcp"
      auth:
        type: "bearer"
        token: "${MCP_API_TOKEN}"
      timeout: 30s
      retry:
        max_attempts: 3
        backoff: "exponential"
    
    realtime-data:
      transport: "websocket"
      url: "wss://realtime.example.com/mcp"
      auth:
        type: "api_key"
        key: "${MCP_WS_KEY}"
```

### Phase 2: WebSocket Real-time Support (3-4 weeks)

#### 2.1 WebSocket Transport Implementation
```rust
pub struct WebSocketTransport {
    ws_url: String,
    auth_config: Option<AuthConfig>,
    connection: Arc<Mutex<Option<WebSocketStream<MaybeTlsStream<TcpStream>>>>>,
    message_handlers: Arc<RwLock<HashMap<String, mpsc::UnboundedSender<JsonRpcResponse>>>>,
    notification_tx: mpsc::UnboundedSender<JsonRpcNotification>,
}

impl WebSocketTransport {
    async fn connect(&mut self) -> Result<()> {
        let (ws_stream, _) = connect_async(&self.ws_url).await?;
        *self.connection.lock().await = Some(ws_stream);
        self.start_message_loop().await?;
        Ok(())
    }
    
    async fn start_message_loop(&self) -> Result<()> {
        // Handle incoming messages and route to appropriate handlers
    }
}
```

#### 2.2 Resource Subscription System
```rust
pub struct ResourceSubscription {
    pub resource_uri: String,
    pub subscription_id: String,
    pub callback: Box<dyn Fn(ResourceUpdate) -> Result<()> + Send + Sync>,
}

pub struct ResourceManager {
    subscriptions: Arc<RwLock<HashMap<String, ResourceSubscription>>>,
    transport: Arc<dyn McpTransport>,
}

impl ResourceManager {
    pub async fn subscribe_to_resource(
        &self,
        uri: &str,
        callback: impl Fn(ResourceUpdate) -> Result<()> + Send + Sync + 'static,
    ) -> Result<String> {
        let subscription_id = Uuid::new_v4().to_string();
        
        // Send subscription request
        let params = json!({
            "uri": uri,
            "subscriptionId": subscription_id
        });
        
        self.transport.send_request(JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: json!(Uuid::new_v4().to_string()),
            method: "resources/subscribe".to_string(),
            params: Some(params),
        }).await?;
        
        // Store subscription
        let subscription = ResourceSubscription {
            resource_uri: uri.to_string(),
            subscription_id: subscription_id.clone(),
            callback: Box::new(callback),
        };
        
        self.subscriptions.write().await.insert(subscription_id.clone(), subscription);
        Ok(subscription_id)
    }
}
```

### Phase 3: HTTP Server Mode (2-3 weeks)

#### 3.1 MCP HTTP Server Implementation
```rust
use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::post,
    Router,
};

pub struct McpHttpServer {
    adapter: Arc<FluentMcpAdapter>,
    port: u16,
    auth_config: Option<ServerAuthConfig>,
}

impl McpHttpServer {
    pub async fn start(&self) -> Result<()> {
        let app = Router::new()
            .route("/mcp", post(handle_mcp_request))
            .route("/mcp/ws", axum::routing::get(handle_websocket_upgrade))
            .with_state(self.adapter.clone());
            
        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", self.port)).await?;
        axum::serve(listener, app).await?;
        Ok(())
    }
}

async fn handle_mcp_request(
    State(adapter): State<Arc<FluentMcpAdapter>>,
    Json(request): Json<JsonRpcRequest>,
) -> Result<Json<JsonRpcResponse>, StatusCode> {
    // Process MCP request and return response
}
```

### Phase 4: Performance Optimization (2-3 weeks)

#### 4.1 Connection Pooling
```rust
pub struct HttpConnectionPool {
    pool: Arc<Mutex<Vec<reqwest::Client>>>,
    max_connections: usize,
    connection_timeout: Duration,
}

impl HttpConnectionPool {
    pub async fn get_client(&self) -> Result<reqwest::Client> {
        // Implement connection pooling logic
    }
}
```

#### 4.2 Request Batching
```rust
pub struct BatchRequestManager {
    pending_requests: Arc<Mutex<Vec<JsonRpcRequest>>>,
    batch_size: usize,
    batch_timeout: Duration,
}

impl BatchRequestManager {
    pub async fn add_request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
        // Implement request batching logic
    }
}
```

## Integration Points

### 1. McpClientManager Enhancement
```rust
impl McpClientManager {
    pub async fn connect_http_server(
        &mut self,
        name: String,
        url: String,
        auth_config: Option<AuthConfig>,
    ) -> Result<()> {
        let transport = HttpTransport::new(url, auth_config)?;
        let client = McpClient::new(Box::new(transport)).await?;
        self.clients.insert(name, client);
        Ok(())
    }
    
    pub async fn connect_websocket_server(
        &mut self,
        name: String,
        ws_url: String,
        auth_config: Option<AuthConfig>,
    ) -> Result<()> {
        let transport = WebSocketTransport::new(ws_url, auth_config)?;
        let client = McpClient::new(Box::new(transport)).await?;
        self.clients.insert(name, client);
        Ok(())
    }
}
```

### 2. CLI Command Extensions
```bash
# HTTP MCP server connection
fluent openai agent-mcp \
  --task "analyze data" \
  --http-servers "analytics:https://api.analytics.com/mcp" \
  --auth-token "${ANALYTICS_TOKEN}"

# WebSocket real-time connection
fluent openai agent-mcp \
  --task "monitor system" \
  --ws-servers "monitoring:wss://monitor.example.com/mcp" \
  --subscribe-resources "system/metrics,alerts/critical"

# Start HTTP MCP server
fluent openai mcp-server \
  --transport http \
  --port 8080 \
  --auth bearer \
  --token-file /etc/mcp/tokens.txt
```

## Risk Assessment and Mitigation

### High-Risk Areas
1. **WebSocket Connection Management**: Complex state management
   - **Mitigation**: Comprehensive reconnection logic, circuit breakers
2. **Authentication Security**: Token management and validation
   - **Mitigation**: Secure token storage, rotation mechanisms
3. **Performance Impact**: Network latency vs STDIO speed
   - **Mitigation**: Connection pooling, request batching, caching

### Medium-Risk Areas
1. **Protocol Compatibility**: HTTP vs STDIO differences
   - **Mitigation**: Comprehensive test suite, protocol validation
2. **Resource Subscription Complexity**: Real-time state synchronization
   - **Mitigation**: Event sourcing patterns, conflict resolution

## Implementation Milestones

### Milestone 1: HTTP Transport Foundation (Week 1-2)
- [ ] Transport abstraction layer
- [ ] Basic HTTP transport implementation
- [ ] Configuration system updates
- [ ] Unit tests for HTTP transport

### Milestone 2: WebSocket Integration (Week 3-4)
- [ ] WebSocket transport implementation
- [ ] Real-time message handling
- [ ] Resource subscription system
- [ ] Integration tests

### Milestone 3: Server Mode (Week 5-6)
- [ ] HTTP server implementation
- [ ] WebSocket server support
- [ ] Authentication and authorization
- [ ] Performance benchmarks

### Milestone 4: Production Readiness (Week 7-8)
- [ ] Connection pooling and optimization
- [ ] Comprehensive error handling
- [ ] Documentation and examples
- [ ] Security audit and testing

## Success Metrics

### Technical Metrics
- **Latency**: HTTP requests < 100ms p95, WebSocket < 10ms
- **Throughput**: Support 1000+ concurrent connections
- **Reliability**: 99.9% uptime for server mode
- **Compatibility**: 100% MCP protocol compliance

### Functional Metrics
- **Transport Flexibility**: Support STDIO, HTTP, WebSocket
- **Real-time Capabilities**: Sub-second resource updates
- **Scalability**: Horizontal scaling support
- **Security**: Enterprise-grade authentication and authorization

## Estimated Effort

**Total Effort**: 11-16 weeks
- **Development**: 8-12 weeks (2 senior developers)
- **Testing**: 2-3 weeks
- **Documentation**: 1 week

**Complexity**: High
- **Technical Complexity**: Network programming, real-time systems
- **Integration Complexity**: Multiple transport protocols
- **Testing Complexity**: Network simulation, load testing

This implementation will establish Fluent CLI as a comprehensive MCP platform supporting all major transport protocols and enabling enterprise-scale deployments.
