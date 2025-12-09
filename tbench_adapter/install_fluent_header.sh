#!/bin/bash
# Self-extracting Fluent CLI installer for Terminal-Bench
# Note: This script is sourced by terminal-bench, so $0 will be /bin/bash
set -e

INSTALL_DIR="/app"
BINARY_PATH="$INSTALL_DIR/fluent"
CONFIG_PATH="$INSTALL_DIR/fluent_config.toml"
AGENT_CONFIG_PATH="$INSTALL_DIR/agent_config.json"

echo "Installing Fluent CLI to $INSTALL_DIR..."
mkdir -p "$INSTALL_DIR"

# Create TOML config file using [[engines]] array format
# Note: Using quoted heredoc to preserve ${VAR} syntax for runtime expansion by fluent config loader
cat > "$CONFIG_PATH" << 'CONFIGEOF'
[[engines]]
name = "claude-sonnet"
engine = "anthropic"

[engines.connection]
protocol = "https"
hostname = "api.anthropic.com"
port = 443
request_path = "/v1/messages"

[engines.parameters]
bearer_token = "${ANTHROPIC_API_KEY}"
modelName = "claude-sonnet-4-20250514"
temperature = 0.1
max_tokens = 16000
system = "You are an expert AI assistant helping to solve coding tasks. Analyze problems carefully, write correct code, and verify your solutions work."
CONFIGEOF

# Create JSON agent config with required fields
cat > "$AGENT_CONFIG_PATH" << 'AGENTEOF'
{
  "agent": {
    "reasoning_engine": "claude-sonnet",
    "action_engine": "claude-sonnet",
    "reflection_engine": "claude-sonnet",
    "memory_database": "sqlite:///app/agent_memory.db",
    "tools": {
      "file_operations": true,
      "shell_commands": true,
      "rust_compiler": false,
      "git_operations": false,
      "allowed_paths": ["/app", "/tmp", "/home", "/root", "/var", "/etc", "/usr"],
      "allowed_commands": ["*"]
    },
    "config_path": "/app/fluent_config.toml",
    "max_iterations": 50,
    "timeout_seconds": 3600
  }
}
AGENTEOF

# Extract embedded binary
echo "Extracting binary..."

# IMPORTANT: Always use the hardcoded path because this script is sourced,
# which means $0 is /bin/bash, not the actual script path
SCRIPT_PATH="/installed-agent/install-agent.sh"

if [ ! -f "$SCRIPT_PATH" ]; then
    echo "ERROR: Install script not found at $SCRIPT_PATH"
    return 1 2>/dev/null || true
fi

echo "Script path: $SCRIPT_PATH"
echo "Script size: $(wc -c < "$SCRIPT_PATH") bytes"

# Find the marker line
MARKER_LINE=$(grep -n '^__BINARY_DATA_START__$' "$SCRIPT_PATH" | cut -d: -f1 | head -1)
echo "Marker found at line: ${MARKER_LINE:-not found}"

if [ -z "$MARKER_LINE" ]; then
    echo "ERROR: Binary marker not found in script"
    return 1 2>/dev/null || true
fi

# Extract everything after the marker line and base64 decode
BINARY_START=$((MARKER_LINE + 1))
echo "Extracting binary data starting at line $BINARY_START..."
tail -n +"$BINARY_START" "$SCRIPT_PATH" | base64 -d > "$BINARY_PATH"

# Verify extraction
BINARY_SIZE=$(wc -c < "$BINARY_PATH")
echo "Binary extracted: $BINARY_SIZE bytes"

if [ "$BINARY_SIZE" -lt 1000 ]; then
    echo "ERROR: Binary extraction failed (file too small)"
    return 1 2>/dev/null || true
fi

chmod +x "$BINARY_PATH"
echo "Fluent CLI installed successfully!"
ls -la "$BINARY_PATH"

# Test the binary
"$BINARY_PATH" --version || echo "Warning: Binary may need additional dependencies"

return 0 2>/dev/null || true
__BINARY_DATA_START__
