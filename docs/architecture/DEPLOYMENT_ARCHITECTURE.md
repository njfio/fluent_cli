# Deployment Architecture

## Overview

This document outlines the various deployment patterns, infrastructure requirements, and operational considerations for the Fluent CLI system across different environments and use cases.

## Deployment Models

### 1. Local Development Environment

```
┌─────────────────────────────────────────────────────────────────┐
│                    Developer Workstation                        │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────┐ │
│  │ Fluent CLI  │  │   SQLite    │  │    Logs     │  │ Config  │ │
│  │   Binary    │  │  Database   │  │    Files    │  │  Files  │ │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────┘ │
└─────────────────────────────────────────────────────────────────┘
                                │
                    ┌─────────────────────┐
                    │   External APIs     │
                    ├─────────────────────┤
                    │ • OpenAI API        │
                    │ • Anthropic API     │
                    │ • Google Gemini     │
                    │ • Other LLM APIs    │
                    └─────────────────────┘
```

**Components**:
- Single binary executable
- Local SQLite database for memory/cache
- Configuration files (YAML)
- Log files for debugging
- Direct API connections to LLM providers

**Requirements**:
- Rust toolchain for building from source
- Network access for API calls
- File system permissions for data storage
- Environment variables for API keys

### 2. Enterprise Desktop Deployment

```
┌─────────────────────────────────────────────────────────────────┐
│                    Enterprise Workstation                       │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────┐ │
│  │ Fluent CLI  │  │   Shared    │  │  Centralized│  │ Policy  │ │
│  │   Binary    │  │  Database   │  │   Logging   │  │ Config  │ │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────┘ │
└─────────────────────────────────────────────────────────────────┘
                                │
                    ┌─────────────────────┐
                    │ Enterprise Gateway  │
                    ├─────────────────────┤
                    │ • API Rate Limiting │
                    │ • Security Scanning │
                    │ • Audit Logging     │
                    │ • Cost Management   │
                    └─────────────────────┘
```

**Features**:
- Centralized configuration management
- Shared database for team collaboration
- Enterprise security policies
- Audit logging and compliance
- API gateway for cost control

### 3. Server Deployment (MCP Server Mode)

```
┌─────────────────────────────────────────────────────────────────┐
│                        Server Environment                       │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────┐ │
│  │ Fluent MCP  │  │ PostgreSQL  │  │    Redis    │  │  Nginx  │ │
│  │   Server    │  │  Database   │  │    Cache    │  │ Reverse │ │
│  │             │  │             │  │             │  │  Proxy  │ │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────┘ │
└─────────────────────────────────────────────────────────────────┘
                                │
                    ┌─────────────────────┐
                    │    MCP Clients      │
                    ├─────────────────────┤
                    │ • Claude Desktop    │
                    │ • VS Code Extension │
                    │ • Custom Clients    │
                    │ • Other AI Tools    │
                    └─────────────────────┘
```

**Configuration**:
```yaml
# server-config.yaml
server:
  mode: "mcp"
  transport: "http"
  port: 8080
  host: "0.0.0.0"
  
database:
  type: "postgresql"
  url: "postgresql://user:pass@localhost:5432/fluent"
  
cache:
  type: "redis"
  url: "redis://localhost:6379"
  
security:
  tls_enabled: true
  cert_file: "/etc/ssl/certs/fluent.crt"
  key_file: "/etc/ssl/private/fluent.key"
```

### 4. Container Deployment (Docker)

```dockerfile
# Dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/fluent /usr/local/bin/
COPY --from=builder /app/config.yaml /etc/fluent/

EXPOSE 8080
CMD ["fluent", "mcp", "--config", "/etc/fluent/config.yaml"]
```

**Docker Compose Setup**:
```yaml
# docker-compose.yml
version: '3.8'
services:
  fluent-cli:
    build: .
    ports:
      - "8080:8080"
    environment:
      - OPENAI_API_KEY=${OPENAI_API_KEY}
      - ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY}
    volumes:
      - ./config:/etc/fluent
      - ./data:/var/lib/fluent
    depends_on:
      - postgres
      - redis

  postgres:
    image: postgres:15
    environment:
      - POSTGRES_DB=fluent
      - POSTGRES_USER=fluent
      - POSTGRES_PASSWORD=password
    volumes:
      - postgres_data:/var/lib/postgresql/data

  redis:
    image: redis:7-alpine
    volumes:
      - redis_data:/data

volumes:
  postgres_data:
  redis_data:
```

### 5. Kubernetes Deployment

```yaml
# k8s-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: fluent-cli
  labels:
    app: fluent-cli
spec:
  replicas: 3
  selector:
    matchLabels:
      app: fluent-cli
  template:
    metadata:
      labels:
        app: fluent-cli
    spec:
      containers:
      - name: fluent-cli
        image: fluent-cli:latest
        ports:
        - containerPort: 8080
        env:
        - name: OPENAI_API_KEY
          valueFrom:
            secretKeyRef:
              name: api-keys
              key: openai
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: database
              key: url
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "512Mi"
            cpu: "500m"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /ready
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5

---
apiVersion: v1
kind: Service
metadata:
  name: fluent-cli-service
spec:
  selector:
    app: fluent-cli
  ports:
  - protocol: TCP
    port: 80
    targetPort: 8080
  type: LoadBalancer
```

### 6. AWS Lambda Deployment

```rust
// lambda-handler.rs
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use serde::{Deserialize, Serialize};
use fluent_cli::cli;

#[derive(Deserialize)]
struct Request {
    engine: String,
    prompt: String,
    config: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct Response {
    content: String,
    usage: serde_json::Value,
    cost: f64,
}

async fn function_handler(event: LambdaEvent<Request>) -> Result<Response, Error> {
    let (event, _context) = event.into_parts();
    
    // Execute fluent CLI logic
    let result = cli::execute_request(&event.engine, &event.prompt, event.config).await?;
    
    Ok(Response {
        content: result.content,
        usage: serde_json::to_value(result.usage)?,
        cost: result.cost.total_cost,
    })
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(service_fn(function_handler)).await
}
```

**Lambda Configuration**:
```yaml
# serverless.yml
service: fluent-cli-lambda

provider:
  name: aws
  runtime: provided.al2
  region: us-east-1
  environment:
    OPENAI_API_KEY: ${env:OPENAI_API_KEY}
    ANTHROPIC_API_KEY: ${env:ANTHROPIC_API_KEY}

functions:
  fluent:
    handler: bootstrap
    timeout: 300
    memorySize: 1024
    events:
      - http:
          path: /execute
          method: post
```

## Infrastructure Requirements

### 1. Compute Resources

| Deployment Type | CPU | Memory | Storage | Network |
|----------------|-----|--------|---------|---------|
| Local Dev | 1 core | 512MB | 1GB | Broadband |
| Enterprise | 2 cores | 2GB | 10GB | Corporate |
| Server | 4 cores | 4GB | 50GB | High-speed |
| Container | 2 cores | 1GB | 20GB | Cloud |
| Lambda | Variable | 1GB | Ephemeral | AWS |

### 2. Database Requirements

**SQLite (Local)**:
- File-based storage
- No additional infrastructure
- Suitable for single-user scenarios

**PostgreSQL (Server)**:
- Dedicated database server
- Connection pooling
- Backup and recovery
- High availability options

**Redis (Caching)**:
- In-memory caching
- Session storage
- Rate limiting data
- Performance optimization

### 3. Security Considerations

**API Key Management**:
```bash
# Environment variables
export OPENAI_API_KEY="sk-..."
export ANTHROPIC_API_KEY="sk-ant-..."

# Kubernetes secrets
kubectl create secret generic api-keys \
  --from-literal=openai="sk-..." \
  --from-literal=anthropic="sk-ant-..."

# AWS Secrets Manager
aws secretsmanager create-secret \
  --name fluent-cli/api-keys \
  --secret-string '{"openai":"sk-...","anthropic":"sk-ant-..."}'
```

**Network Security**:
- TLS encryption for all communications
- API rate limiting and throttling
- IP whitelisting for enterprise deployments
- VPN access for sensitive environments

### 4. Monitoring and Observability

**Logging Configuration**:
```yaml
logging:
  level: "info"
  format: "json"
  outputs:
    - type: "file"
      path: "/var/log/fluent/app.log"
    - type: "stdout"
      format: "human"
  
metrics:
  enabled: true
  endpoint: "http://prometheus:9090"
  interval: "30s"
```

**Health Checks**:
```rust
// Health check endpoint
async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION"),
        "uptime": get_uptime(),
        "database": check_database_connection().await,
        "apis": check_api_connectivity().await,
    }))
}
```

## Operational Procedures

### 1. Deployment Process
1. **Build**: Compile optimized release binary
2. **Test**: Run integration tests
3. **Package**: Create deployment artifacts
4. **Deploy**: Deploy to target environment
5. **Verify**: Run health checks
6. **Monitor**: Watch metrics and logs

### 2. Backup and Recovery
- **Configuration**: Version-controlled YAML files
- **Database**: Regular automated backups
- **Logs**: Centralized log aggregation
- **Secrets**: Secure secret management

### 3. Scaling Strategies
- **Horizontal**: Multiple server instances
- **Vertical**: Increased resource allocation
- **Auto-scaling**: Dynamic resource adjustment
- **Load balancing**: Traffic distribution

This deployment architecture provides comprehensive guidance for deploying Fluent CLI across various environments while maintaining security, scalability, and operational excellence.
