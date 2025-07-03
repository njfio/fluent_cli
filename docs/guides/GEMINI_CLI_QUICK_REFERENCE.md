# Gemini CLI Quick Reference

## Essential Commands

### Installation & Setup
```bash
# Install globally
npm install -g @google/gemini-cli

# Run without installation
npx https://github.com/google-gemini/gemini-cli

# Set API key (optional, for higher limits)
export GEMINI_API_KEY="your-api-key-here"

# Check version
gemini --version
```

### Basic Usage
```bash
# Interactive mode
gemini

# Non-interactive mode
gemini -p "your prompt here"

# YOLO mode (auto-accept all actions)
gemini -y -p "fix all linting errors"

# Include all files in context
gemini -a -p "analyze entire codebase"

# Debug mode
gemini -d -p "investigate complex issue"
```

### Interactive Commands
```bash
# In interactive session:
/memory     # View conversation memory
/stats      # Show usage statistics
/tools      # List available tools
/mcp        # Manage MCP servers
/theme      # Change color theme
/chat       # Switch to chat mode
/editor     # Open editor mode
/compress   # Compress conversation history
```

## Command Line Options

### Core Options
```bash
-p, --prompt           # Prompt text
-m, --model           # Model selection (default: gemini-2.5-pro)
-y, --yolo            # Auto-accept all actions
-a, --all_files       # Include ALL files in context
-d, --debug           # Debug mode
-s, --sandbox         # Run in sandbox
-c, --checkpointing   # Enable file edit checkpointing
-v, --version         # Show version
-h, --help            # Show help
```

### Advanced Options
```bash
--show_memory_usage    # Show memory usage in status bar
--telemetry           # Enable telemetry
--sandbox-image       # Custom sandbox image URI
```

## Common Task Patterns

### Code Analysis & Review
```bash
# Security audit
gemini -p "audit this codebase for security vulnerabilities"

# Performance analysis
gemini -p "identify performance bottlenecks and suggest optimizations"

# Code quality review
gemini -p "review code quality and suggest improvements"

# Architecture analysis
gemini -p "analyze the system architecture and suggest improvements"

# Specific file review
gemini -p "review src/auth.rs for OAuth 2.1 compliance"
```

### Bug Fixing & Testing
```bash
# Fix compilation errors
gemini -y -p "fix all compilation errors"

# Debug specific issues
gemini -p "investigate why the authentication is failing"

# Generate tests
gemini -p "generate comprehensive tests for the crypto module"

# Run and fix tests
gemini -p "run cargo test and fix any failing tests"

# Performance debugging
gemini -p "profile the application and fix bottlenecks"
```

### Code Generation & Implementation
```bash
# Implement new features
gemini -p "implement OAuth 2.1 authentication following existing patterns"

# Refactor code
gemini -p "refactor handlers to use modern async/await patterns"

# Generate documentation
gemini -p "create API documentation for all public functions"

# Add missing functionality
gemini -p "implement all TODO items in the codebase"

# Create new modules
gemini -p "create a new cryptographic module for ZKP operations"
```

### Project Management
```bash
# Project status
gemini -p "generate a comprehensive project status report"

# Release preparation
gemini -p "prepare for v0.2.0 release: update docs and changelog"

# Dependency management
gemini -p "audit and update all dependencies to latest versions"

# Code cleanup
gemini -p "clean up all TODO comments and unused code"
```

## MCP Server Integration

### Popular MCP Servers
```bash
# Filesystem operations
npm install -g @modelcontextprotocol/server-filesystem

# GitHub integration
npm install -g @modelcontextprotocol/server-github

# Database operations
npm install -g @modelcontextprotocol/server-sqlite

# Web browsing
npm install -g @modelcontextprotocol/server-brave-search
```

### MCP Configuration
```bash
# Create config directory
mkdir -p ~/.config/gemini-cli

# Example MCP config
cat > ~/.config/gemini-cli/mcp.json << EOF
{
  "servers": {
    "filesystem": {
      "command": "npx",
      "args": ["@modelcontextprotocol/server-filesystem", "/path/to/project"]
    },
    "github": {
      "command": "npx",
      "args": ["@modelcontextprotocol/server-github"],
      "env": {
        "GITHUB_TOKEN": "your-token"
      }
    }
  }
}
EOF
```

### Using MCP Servers
```bash
# List MCP servers
gemini
> /mcp

# File operations via MCP
gemini -p "organize all PDF files by date using filesystem MCP"

# GitHub operations
gemini -p "create a PR for recent changes using GitHub MCP"

# Database queries
gemini -p "analyze user data using SQLite MCP"
```

## ZKP Framework Specific

### Cryptographic Tasks
```bash
# Security audit
gemini -p "audit ZKP implementation for cryptographic vulnerabilities"

# Constant-time verification
gemini -p "verify all crypto operations are constant-time"

# Circuit optimization
gemini -p "optimize Poseidon gadget constraints for efficiency"

# Side-channel analysis
gemini -p "analyze code for potential side-channel attacks"
```

### Testing & Validation
```bash
# Generate crypto tests
gemini -p "generate test vectors for all cryptographic primitives"

# Integration testing
gemini -p "create end-to-end tests for ZKP proof generation"

# Performance benchmarks
gemini -p "create benchmarks for all crypto operations"

# Property-based testing
gemini -p "generate property-based tests for circuit constraints"
```

### API Development
```bash
# REST API implementation
gemini -p "implement secure REST endpoints for proof generation"

# OAuth integration
gemini -p "implement OAuth 2.1 with multiple social providers"

# API documentation
gemini -p "generate OpenAPI docs with examples"

# Authentication middleware
gemini -p "implement JWT authentication middleware"
```

## Automation Scripts

### Basic Automation
```bash
#!/bin/bash
# Simple automation script

echo "Running automated code review..."
gemini -p "review codebase for issues" > review-report.txt

echo "Fixing linting issues..."
gemini -y -p "fix all linting errors"

echo "Running tests..."
gemini -p "run test suite and report results"
```

### Quality Pipeline
```bash
#!/bin/bash
# quality-pipeline.sh

echo "Security scan..."
gemini -p "security audit" > security-report.txt

echo "Performance analysis..."
gemini -p "performance analysis" > perf-report.txt

echo "Code quality check..."
gemini -p "code quality review" > quality-report.txt

echo "Test coverage..."
gemini -p "analyze test coverage" > coverage-report.txt
```

### CI/CD Integration
```bash
# GitHub Actions workflow
name: Gemini Review
on: [pull_request]
jobs:
  review:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Setup Node.js
        uses: actions/setup-node@v3
        with:
          node-version: '18'
      - name: Install Gemini CLI
        run: npm install -g @google/gemini-cli
      - name: Run Review
        env:
          GEMINI_API_KEY: ${{ secrets.GEMINI_API_KEY }}
        run: gemini -p "review PR changes" > review.txt
```

## Best Practices

### Effective Prompting
```bash
# Be specific
gemini -p "fix Poseidon hash arity issue in zkp-core/src/crypto.rs"

# Provide context
gemini -p "implementing OAuth 2.1 - ensure PKCE compliance"

# Break complex tasks
gemini -p "first analyze auth flow, then suggest improvements"

# Use constraints
gemini -p "refactor without changing public API"
```

### Safety & Security
```bash
# Always backup before major changes
git commit -am "backup before automated changes"

# Use sandbox for experimental work
gemini -s -p "test experimental crypto implementation"

# Review generated code
git diff HEAD~1

# Use checkpointing for long tasks
gemini -c -p "extensive refactoring with incremental saves"
```

### Performance Tips
```bash
# Use specific models
gemini -m "gemini-2.5-pro" -p "complex analysis"

# Limit context when possible
gemini -p "analyze only the auth module"

# Use YOLO for safe tasks
gemini -y -p "format code and fix linting"

# Compress memory when needed
gemini
> /compress
```

## Memory Management

### Context Control
```bash
# View memory usage
gemini
> /memory

# Include all files (use carefully)
gemini -a -p "comprehensive codebase analysis"

# Focus on specific files
gemini -p "analyze only src/crypto.rs and src/auth.rs"

# Compress conversation
gemini
> /compress
```

### Session Management
```bash
# Start fresh for new tasks
gemini -p "summarize progress and start new focused session"

# Use checkpointing for long tasks
gemini -c -p "perform extensive refactoring"

# Monitor memory usage
gemini --show_memory_usage
```

## Troubleshooting

### Common Issues
```bash
# Authentication problems
export GEMINI_API_KEY="your-key"
gemini -p "test prompt"

# Memory issues
gemini
> /compress
> /memory

# Debug mode for issues
gemini -d -p "debug this problem"

# Check version and update
gemini --version
npm update -g @google/gemini-cli
```

### Performance Issues
```bash
# Use specific context
gemini -p "analyze only the failing module"

# Enable debug mode
gemini -d -p "investigate performance issue"

# Use sandbox for isolation
gemini -s -p "test in isolated environment"
```

## Environment Setup

### Authentication
```bash
# Personal account (free tier)
gemini  # Follow auth flow

# API key (higher limits)
export GEMINI_API_KEY="your-key"

# Workspace account
# See documentation for enterprise setup
```

### Configuration
```bash
# Create config directory
mkdir -p ~/.config/gemini-cli

# Set up MCP servers
# Edit ~/.config/gemini-cli/mcp.json

# Verify setup
gemini -p "hello world test"
```

This quick reference provides immediate access to the most commonly used Gemini CLI commands and patterns for efficient task offloading.
