# Claude Code Quick Reference

## Essential Commands

### Basic Usage
```bash
# Interactive mode
claude

# Non-interactive (print mode)
claude -p "your prompt here"

# Continue previous session
claude --continue

# Resume specific session
claude --resume
```

### File Operations
```bash
# Reference specific files
claude -p "explain @src/main.rs"

# Reference directories
claude -p "analyze @src/handlers/"

# Process file through stdin
cat file.rs | claude -p "review this code"
```

### Output Formats
```bash
# JSON output (for scripting)
claude -p "analyze code" --output-format json

# Streaming JSON
claude -p "build feature" --output-format stream-json

# Text output (default)
claude -p "explain this" --output-format text
```

## Common Task Patterns

### Code Review & Analysis
```bash
# Security review
claude -p "scan for security vulnerabilities in this codebase"

# Performance analysis
claude -p "identify performance bottlenecks and suggest optimizations"

# Code quality check
claude -p "review code quality and suggest improvements"

# Architecture analysis
claude -p "analyze the overall architecture and suggest improvements"
```

### Bug Fixing & Testing
```bash
# Fix compilation errors
claude -p "fix all compilation errors"

# Run and fix tests
claude -p "run cargo test and fix any failing tests"

# Generate tests
claude -p "generate comprehensive tests for @src/module.rs"

# Debug specific issue
claude -p "investigate why the authentication is failing"
```

### Code Generation & Refactoring
```bash
# Generate new module
claude -p "implement a new OAuth module following existing patterns"

# Refactor code
claude -p "refactor @src/handlers.rs to use modern Rust patterns"

# Add documentation
claude -p "add comprehensive documentation to all public functions"

# Optimize performance
claude -p "optimize @src/crypto.rs for better performance"
```

### Project Management
```bash
# Create PR
claude -p "create a PR for the recent changes"

# Update documentation
claude -p "update README.md to reflect new features"

# Generate release notes
claude -p "generate release notes based on recent commits"

# Project status
claude -p "provide a comprehensive project status report"
```

## Advanced Features

### Custom Commands
```bash
# Create project command
mkdir -p .claude/commands
echo "Review this code for security issues:" > .claude/commands/security.md

# Use project command
claude
> /project:security

# Create personal command
mkdir -p ~/.claude/commands
echo "Optimize this code for performance:" > ~/.claude/commands/optimize.md

# Use personal command
claude
> /user:optimize
```

### Session Management
```bash
# Continue with new prompt
claude --continue -p "now implement the suggested changes"

# Resume with session ID
claude --resume abc123-def456

# Non-interactive continue
claude -p --continue "run the tests again"
```

### Tool Control
```bash
# Allow specific tools
claude -p "task" --allowedTools "Read,Write,Bash"

# Disallow tools
claude -p "task" --disallowedTools "Bash"

# Permission modes
claude --permission-mode plan  # Review before execution
claude --permission-mode acceptEdits  # Auto-accept edits
```

## ZKP Framework Specific

### Cryptographic Code
```bash
# Security audit
claude -p "audit @zkp-core/src/crypto.rs for cryptographic vulnerabilities"

# Constant-time analysis
claude -p "verify constant-time operations in @zkp-circuits/"

# Circuit optimization
claude -p "optimize constraints in @zkp-circuits/src/gadgets/"
```

### Testing & Validation
```bash
# Generate crypto tests
claude -p "generate comprehensive tests for Poseidon hash implementation"

# Integration testing
claude -p "create end-to-end tests for the ZKP proof generation"

# Performance benchmarks
claude -p "create performance benchmarks for cryptographic operations"
```

### API Development
```bash
# REST API implementation
claude -p "implement REST endpoints for proof generation"

# OAuth integration
claude -p "implement OAuth 2.1 compliant authentication"

# API documentation
claude -p "generate OpenAPI documentation for all endpoints"
```

## Automation Scripts

### Basic Automation
```bash
#!/bin/bash
# Simple automation script

# Run code review
claude -p "review recent changes for issues" --output-format json > review.json

# Check if issues found
if jq -e '.result | contains("issue")' review.json > /dev/null; then
    echo "Issues found, please review"
    exit 1
fi
```

### Quality Pipeline
```bash
#!/bin/bash
# quality-check.sh

echo "Running quality checks..."

# Security scan
claude -p "security scan" --output-format json > security.json

# Performance check
claude -p "performance analysis" --output-format json > performance.json

# Test coverage
claude -p "analyze test coverage" --output-format json > coverage.json

echo "Quality checks complete"
```

### CI/CD Integration
```bash
# In GitHub Actions
- name: Claude Code Review
  run: |
    claude -p "review PR changes" --output-format json > results.json
    # Process results...
```

## Best Practices

### Effective Prompting
```bash
# Be specific
claude -p "fix the Poseidon hash arity issue in zkp-core/src/crypto.rs"

# Provide context
claude -p "implementing OAuth 2.1 - review @src/oauth.rs for compliance"

# Use file references
claude -p "explain the relationship between @src/handlers.rs and @src/auth.rs"
```

### Error Handling
```bash
# Check exit codes
if ! claude -p "run tests" 2>error.log; then
    echo "Command failed"
    cat error.log
fi

# Retry logic
for i in {1..3}; do
    claude -p "task" && break
    echo "Retry $i failed"
done
```

### Cost Management
```bash
# Limit turns
claude -p "quick review" --max-turns 3

# Use JSON to track costs
claude -p "task" --output-format json | jq '.total_cost_usd'
```

## Troubleshooting

### Common Issues
```bash
# Verbose output
claude --verbose -p "debug task"

# Check version
claude --version

# Update Claude Code
npm update -g @anthropic-ai/claude-code
```

### Performance Tips
```bash
# Use specific file references
claude -p "review @specific/file.rs"

# Break large tasks into chunks
claude -p "first analyze architecture, then we'll look at modules"

# Use appropriate output format
claude -p "simple task" --output-format text  # Faster
claude -p "complex task" --output-format json  # More data
```

## Environment Setup

### Authentication
```bash
# Set API key
export ANTHROPIC_API_KEY="your-key-here"

# Verify setup
claude -p "hello" --output-format json
```

### Configuration
```bash
# Create config directory
mkdir -p ~/.claude

# Set up project commands
mkdir -p .claude/commands

# Create MCP config (if needed)
cat > mcp-config.json << EOF
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/path"]
    }
  }
}
EOF
```

This quick reference provides immediate access to the most commonly used Claude Code commands and patterns for efficient task offloading.
