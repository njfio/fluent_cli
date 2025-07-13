# MCP CLI Management Commands

This document provides comprehensive documentation for the Model Context Protocol (MCP) management commands in Fluent CLI.

## Overview

The MCP CLI management system provides production-ready commands for:
- **Server Management**: Start and configure MCP servers
- **Client Operations**: Connect to and manage MCP servers
- **Tool Discovery**: List and execute available tools
- **System Monitoring**: Health checks and performance metrics
- **Configuration Management**: Configure and manage MCP settings

## Command Structure

All MCP commands follow the pattern:
```bash
fluent mcp <subcommand> [options]
```

## Available Commands

### 1. Server Management

#### Start MCP Server
```bash
fluent mcp server [options]
```

**Options:**
- `--port, -p <PORT>`: Port for HTTP transport (optional)
- `--stdio`: Use STDIO transport (default)
- `--config, -c <FILE>`: Configuration file path

**Examples:**
```bash
# Start server with STDIO transport (default)
fluent mcp server --stdio

# Start server with HTTP transport on port 8080
fluent mcp server --port 8080

# Start server with custom configuration
fluent mcp server --config /path/to/mcp-config.toml
```

### 2. Client Operations

#### Connect to MCP Server
```bash
fluent mcp connect --name <NAME> --command <COMMAND> [--args <ARGS>...]
```

**Options:**
- `--name, -n <NAME>`: Server name (required)
- `--command, -c <COMMAND>`: Server command to execute (required)
- `--args, -a <ARGS>`: Command arguments (optional, multiple)

**Examples:**
```bash
# Connect to filesystem server
fluent mcp connect --name filesystem --command mcp-server-filesystem

# Connect to git server with arguments
fluent mcp connect --name git --command mcp-server-git --args --repo /path/to/repo
```

#### Disconnect from MCP Server
```bash
fluent mcp disconnect --name <NAME>
```

**Examples:**
```bash
# Disconnect from filesystem server
fluent mcp disconnect --name filesystem
```

### 3. Tool Management

#### List Available Tools
```bash
fluent mcp tools [options]
```

**Options:**
- `--server, -s <SERVER>`: Filter by server name
- `--json`: Output in JSON format

**Examples:**
```bash
# List all tools from all connected servers
fluent mcp tools

# List tools from specific server
fluent mcp tools --server filesystem

# Get JSON output for programmatic use
fluent mcp tools --json
```

#### Execute Tool
```bash
fluent mcp execute --tool <TOOL> [options]
```

**Options:**
- `--tool, -t <TOOL>`: Tool name to execute (required)
- `--parameters, -p <JSON>`: Tool parameters as JSON (default: "{}")
- `--server, -s <SERVER>`: Preferred server name
- `--timeout <SECONDS>`: Execution timeout in seconds (default: 30)

**Examples:**
```bash
# Execute simple tool without parameters
fluent mcp execute --tool list_files

# Execute tool with parameters
fluent mcp execute --tool read_file --parameters '{"path": "/etc/hosts"}'

# Execute with server preference and timeout
fluent mcp execute --tool git_status --server git --timeout 60
```

### 4. System Monitoring

#### Show System Status
```bash
fluent mcp status [options]
```

**Options:**
- `--json`: Output in JSON format
- `--detailed`: Show detailed metrics

**Examples:**
```bash
# Show basic status
fluent mcp status

# Show detailed metrics
fluent mcp status --detailed

# Get JSON output for monitoring systems
fluent mcp status --json
```

### 5. Configuration Management

#### Manage Configuration
```bash
fluent mcp config [options]
```

**Options:**
- `--show`: Show current configuration
- `--set <KEY>`: Configuration key to set
- `--value <VALUE>`: Configuration value
- `--file, -f <FILE>`: Save configuration to file

**Examples:**
```bash
# Show current configuration
fluent mcp config --show

# Set configuration value (future feature)
fluent mcp config --set server.timeout --value 30

# Save configuration to file (future feature)
fluent mcp config --file /path/to/mcp-config.toml
```

### 6. Legacy Agent Integration

#### Run Agent with MCP (Legacy)
```bash
fluent mcp agent --engine <ENGINE> --task <TASK> [--servers <SERVERS>...]
```

**Options:**
- `--engine, -e <ENGINE>`: LLM engine to use (required)
- `--task, -t <TASK>`: Task description (required)
- `--servers, -s <SERVERS>`: MCP servers to use (multiple)

**Examples:**
```bash
# Run agent with MCP integration
fluent mcp agent --engine openai --task "List files in current directory" --servers filesystem

# Use multiple servers
fluent mcp agent --engine anthropic --task "Analyze git repository" --servers filesystem git
```

## Output Formats

### Human-Readable Output
Default output format with emojis and structured information:
```
🔧 Listing available MCP tools...

📡 Server: filesystem
   Tools: 5
   🔧 read_file
      📝 Read contents of a file
   🔧 write_file
      📝 Write contents to a file
```

### JSON Output
Machine-readable format for automation:
```json
{
  "servers": [
    {
      "name": "filesystem",
      "tools": [
        {
          "name": "read_file",
          "description": "Read contents of a file"
        }
      ]
    }
  ]
}
```

## Error Handling

All commands provide comprehensive error handling with:
- **Clear Error Messages**: Human-readable error descriptions
- **Exit Codes**: Proper exit codes for automation
- **Recovery Suggestions**: Actionable suggestions when possible

## Integration with Existing CLI

The MCP commands integrate seamlessly with the existing Fluent CLI:
- **Configuration**: Uses existing config system
- **Logging**: Integrates with CLI logging
- **Error Handling**: Consistent error patterns
- **Help System**: Standard `--help` support

## Future Enhancements

Planned improvements include:
- **Configuration Persistence**: Save/load MCP configurations
- **Server Discovery**: Automatic server discovery
- **Tool Validation**: Parameter validation before execution
- **Batch Operations**: Execute multiple tools in sequence
- **Monitoring Integration**: Prometheus metrics export
