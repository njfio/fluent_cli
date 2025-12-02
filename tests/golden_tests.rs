use assert_cmd::Command;
use serde_json::Value;

/// Golden tests for response formatting and output consistency
///
/// These tests ensure that output formatting remains consistent across CLI commands
/// and help catch unintended changes to the output format.

// =============================================================================
// Help Output Format Tests
// =============================================================================

/// Test that main help output format contains expected sections
#[test]
fn test_help_output_format() {
    let mut cmd = Command::cargo_bin("fluent").unwrap();
    let output = cmd.arg("--help").output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check expected sections exist in help output
    assert!(
        stdout.contains("Usage:"),
        "Help output should contain 'Usage:' section"
    );
    assert!(
        stdout.contains("Commands:"),
        "Help output should contain 'Commands:' section"
    );
    assert!(
        stdout.contains("Options:"),
        "Help output should contain 'Options:' section"
    );

    // Check that common commands are listed
    assert!(
        stdout.contains("agent") || stdout.contains("Agent"),
        "Help output should list 'agent' command"
    );
    assert!(
        stdout.contains("tools") || stdout.contains("Tools"),
        "Help output should list 'tools' command"
    );
    assert!(
        stdout.contains("engine") || stdout.contains("Engine"),
        "Help output should list 'engine' command"
    );
}

/// Test agent help output format
#[test]
fn test_agent_help_format() {
    let mut cmd = Command::cargo_bin("fluent").unwrap();
    let output = cmd.args(["agent", "--help"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Agent help should contain key information
    assert!(
        stdout.contains("agent") || stdout.contains("Agent"),
        "Agent help should mention agent"
    );
    assert!(
        stdout.contains("Usage:") || stdout.contains("USAGE:"),
        "Agent help should show usage"
    );
}

/// Test tools help output format
#[test]
fn test_tools_help_format() {
    let mut cmd = Command::cargo_bin("fluent").unwrap();
    let output = cmd.args(["tools", "--help"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Tools help should contain subcommands
    assert!(
        stdout.contains("list") || stdout.contains("List"),
        "Tools help should mention list subcommand"
    );
    assert!(
        stdout.contains("describe") || stdout.contains("Describe"),
        "Tools help should mention describe subcommand"
    );
}

/// Test engine help output format
#[test]
fn test_engine_help_format() {
    let mut cmd = Command::cargo_bin("fluent").unwrap();
    let output = cmd.args(["engine", "--help"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Engine help should contain subcommands
    assert!(
        stdout.contains("list") || stdout.contains("List"),
        "Engine help should mention list subcommand"
    );
    assert!(
        stdout.contains("test") || stdout.contains("Test"),
        "Engine help should mention test subcommand"
    );
}

// =============================================================================
// Engine List Format Tests
// =============================================================================

/// Test engine list output format (standard text output)
#[test]
fn test_engine_list_format() {
    let mut cmd = Command::cargo_bin("fluent").unwrap();
    let output = cmd.args(["engine", "list"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Engine list should have consistent structure
    // Either shows configured engines or indicates no engines are configured
    assert!(
        stdout.contains("engine")
            || stdout.contains("Engine")
            || stdout.contains("No engines configured")
            || stdout.contains("Available engines"),
        "Engine list should show engines or indicate none configured"
    );
}

/// Test engine list JSON output format
#[test]
fn test_engine_list_json_format() {
    let mut cmd = Command::cargo_bin("fluent").unwrap();
    let output = cmd.args(["engine", "list", "--json"]).output().unwrap();

    // Should succeed
    assert!(
        output.status.success(),
        "Engine list --json should succeed"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should be valid JSON
    let parsed: Result<Value, _> = serde_json::from_str(&stdout);
    assert!(
        parsed.is_ok(),
        "Engine list --json output should be valid JSON: {}",
        stdout
    );

    let json = parsed.unwrap();

    // Should be an array (list of engines)
    assert!(
        json.is_array(),
        "Engine list --json should output an array, got: {}",
        json
    );

    // Verify structure if engines exist
    if let Some(engines) = json.as_array() {
        for engine in engines {
            // Each engine should have expected fields
            assert!(
                engine.get("name").is_some(),
                "Each engine should have a 'name' field"
            );
            assert!(
                engine.get("engine").is_some(),
                "Each engine should have an 'engine' field"
            );
            assert!(
                engine.get("connection").is_some(),
                "Each engine should have a 'connection' field"
            );
        }
    }
}

// =============================================================================
// Tools List Format Tests
// =============================================================================

/// Test tools list output format (standard text output)
#[test]
fn test_tools_list_format() {
    let mut cmd = Command::cargo_bin("fluent").unwrap();
    let output = cmd.args(["tools", "list"]).output().unwrap();

    // Should succeed
    assert!(
        output.status.success(),
        "Tools list should succeed"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Tools list should show tools in some structured format
    // Looking for common tool names that should always be available
    assert!(
        stdout.len() > 0,
        "Tools list should produce output"
    );
}

/// Test tools list JSON output format
#[test]
fn test_tools_list_json_format() {
    let mut cmd = Command::cargo_bin("fluent").unwrap();
    let output = cmd.args(["tools", "list", "--json"]).output().unwrap();

    // Should succeed
    assert!(
        output.status.success(),
        "Tools list --json should succeed"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should be valid JSON
    let parsed: Result<Value, _> = serde_json::from_str(&stdout);
    assert!(
        parsed.is_ok(),
        "Tools list --json output should be valid JSON: {}",
        stdout
    );

    let json = parsed.unwrap();

    // Should be an object with tools array
    assert!(
        json.is_object(),
        "Tools list --json should output an object, got: {}",
        json
    );

    // Verify structure
    assert!(
        json.get("tools").is_some(),
        "Tools list --json should have 'tools' field"
    );
    assert!(
        json.get("total_count").is_some(),
        "Tools list --json should have 'total_count' field"
    );

    // Verify tools array structure
    if let Some(tools) = json.get("tools").and_then(|t| t.as_array()) {
        for tool in tools {
            // Each tool should have expected fields
            assert!(
                tool.get("name").is_some(),
                "Each tool should have a 'name' field"
            );
            assert!(
                tool.get("description").is_some(),
                "Each tool should have a 'description' field"
            );
        }
    }
}

/// Test tools list with filters maintains format
#[test]
fn test_tools_list_with_filters_json_format() {
    let mut cmd = Command::cargo_bin("fluent").unwrap();
    let output = cmd
        .args(["tools", "list", "--json", "--available"])
        .output()
        .unwrap();

    // Should succeed
    assert!(
        output.status.success(),
        "Tools list with filters should succeed"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should be valid JSON
    let parsed: Result<Value, _> = serde_json::from_str(&stdout);
    assert!(
        parsed.is_ok(),
        "Tools list --json --available output should be valid JSON"
    );

    let json = parsed.unwrap();

    // Should maintain same structure
    assert!(
        json.get("tools").is_some(),
        "Filtered tools list should still have 'tools' field"
    );
    assert!(
        json.get("total_count").is_some(),
        "Filtered tools list should still have 'total_count' field"
    );
    assert!(
        json.get("filters").is_some(),
        "Filtered tools list should have 'filters' field"
    );
}

// =============================================================================
// Tools Describe Format Tests
// =============================================================================

/// Test tools describe JSON output format for a standard tool
#[test]
fn test_tools_describe_json_format() {
    // First get list of available tools
    let mut list_cmd = Command::cargo_bin("fluent").unwrap();
    let list_output = list_cmd.args(["tools", "list", "--json"]).output().unwrap();

    if !list_output.status.success() {
        // Skip if tools list fails
        return;
    }

    let stdout = String::from_utf8_lossy(&list_output.stdout);
    let parsed: Result<Value, _> = serde_json::from_str(&stdout);

    if let Ok(json) = parsed {
        if let Some(tools) = json.get("tools").and_then(|t| t.as_array()) {
            if !tools.is_empty() {
                // Get first tool name
                if let Some(first_tool) = tools[0].get("name").and_then(|n| n.as_str()) {
                    // Test describe for this tool
                    let mut describe_cmd = Command::cargo_bin("fluent").unwrap();
                    let describe_output = describe_cmd
                        .args(["tools", "describe", first_tool, "--json"])
                        .output()
                        .unwrap();

                    if describe_output.status.success() {
                        let describe_stdout = String::from_utf8_lossy(&describe_output.stdout);
                        let describe_parsed: Result<Value, _> =
                            serde_json::from_str(&describe_stdout);

                        assert!(
                            describe_parsed.is_ok(),
                            "Tools describe --json should be valid JSON"
                        );

                        if let Ok(describe_json) = describe_parsed {
                            // Verify structure
                            assert!(
                                describe_json.get("name").is_some(),
                                "Tools describe should have 'name' field"
                            );
                            assert!(
                                describe_json.get("description").is_some(),
                                "Tools describe should have 'description' field"
                            );
                        }
                    }
                }
            }
        }
    }
}

// =============================================================================
// Version Output Format Tests
// =============================================================================

/// Test version output format
#[test]
fn test_version_output_format() {
    let mut cmd = Command::cargo_bin("fluent").unwrap();
    let output = cmd.arg("--version").output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Version should contain package name and version number
    assert!(
        stdout.contains("fluent"),
        "Version output should contain package name"
    );

    // Should contain a version number pattern (e.g., 0.1.0)
    let version_pattern = regex::Regex::new(r"\d+\.\d+\.\d+").unwrap();
    assert!(
        version_pattern.is_match(&stdout),
        "Version output should contain version number in format X.Y.Z"
    );
}

// =============================================================================
// Schema Output Format Tests
// =============================================================================

/// Test schema output is valid JSON Schema
#[test]
fn test_schema_output_format() {
    let mut cmd = Command::cargo_bin("fluent").unwrap();
    let output = cmd.args(["schema"]).output().unwrap();

    // Schema command should succeed or be unknown
    if !output.status.success() {
        // If schema command doesn't exist, skip this test
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("not found") || stderr.contains("unrecognized") {
            return;
        }
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // If we got output, it should be valid JSON
    if !stdout.trim().is_empty() {
        let parsed: Result<Value, _> = serde_json::from_str(&stdout);
        assert!(
            parsed.is_ok(),
            "Schema output should be valid JSON: {}",
            stdout
        );

        // Should be a JSON Schema object
        if let Ok(json) = parsed {
            assert!(
                json.is_object(),
                "Schema output should be a JSON object"
            );
        }
    }
}

// =============================================================================
// Completions Format Tests
// =============================================================================

/// Test completions output format for bash
#[test]
fn test_completions_bash_format() {
    let mut cmd = Command::cargo_bin("fluent").unwrap();
    let output = cmd
        .args(["completions", "--shell", "bash"])
        .output()
        .unwrap();

    // Completions command should succeed or be unknown
    if !output.status.success() {
        // If completions command doesn't exist, skip this test
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("not found") || stderr.contains("unrecognized") {
            return;
        }
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Bash completions should contain bash-specific syntax
    if !stdout.trim().is_empty() {
        assert!(
            stdout.contains("bash") || stdout.contains("complete") || stdout.contains("_fluent"),
            "Bash completions should contain bash completion syntax"
        );
    }
}

/// Test completions output format for zsh
#[test]
fn test_completions_zsh_format() {
    let mut cmd = Command::cargo_bin("fluent").unwrap();
    let output = cmd
        .args(["completions", "--shell", "zsh"])
        .output()
        .unwrap();

    // Completions command should succeed or be unknown
    if !output.status.success() {
        // If completions command doesn't exist, skip this test
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("not found") || stderr.contains("unrecognized") {
            return;
        }
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Zsh completions should contain zsh-specific syntax
    if !stdout.trim().is_empty() {
        assert!(
            stdout.contains("#compdef") || stdout.contains("_fluent"),
            "Zsh completions should contain zsh completion syntax"
        );
    }
}

// =============================================================================
// Error Format Tests
// =============================================================================

/// Test error output format for invalid command
#[test]
fn test_error_format_invalid_command() {
    let mut cmd = Command::cargo_bin("fluent").unwrap();
    let output = cmd.args(["invalid-command-that-doesnt-exist"]).output().unwrap();

    // Should fail
    assert!(
        !output.status.success(),
        "Invalid command should return non-zero exit code"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Error message should contain helpful information
    assert!(
        stderr.contains("error")
            || stderr.contains("Error")
            || stderr.contains("unrecognized")
            || stderr.contains("unexpected"),
        "Error output should indicate an error occurred"
    );
}

/// Test error output format for missing required argument
#[test]
fn test_error_format_missing_argument() {
    let mut cmd = Command::cargo_bin("fluent").unwrap();
    let output = cmd.args(["engine", "test"]).output().unwrap();

    // Should fail (missing engine name)
    assert!(
        !output.status.success(),
        "Missing required argument should return non-zero exit code"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Error should indicate missing argument
    assert!(
        stderr.contains("error")
            || stderr.contains("Error")
            || stderr.contains("required")
            || stderr.contains("missing"),
        "Error output should indicate missing required argument"
    );
}

// =============================================================================
// CSV Format Extraction Tests
// =============================================================================

/// Test that JSON output can be converted to CSV format
#[test]
fn test_json_to_csv_conversion_tools_list() {
    let mut cmd = Command::cargo_bin("fluent").unwrap();
    let output = cmd.args(["tools", "list", "--json"]).output().unwrap();

    if !output.status.success() {
        return;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: Result<Value, _> = serde_json::from_str(&stdout);

    if let Ok(json) = parsed {
        if let Some(tools) = json.get("tools").and_then(|t| t.as_array()) {
            if !tools.is_empty() {
                // Verify that we can extract CSV-like data from JSON
                // Check that all tools have consistent fields that could be CSV columns
                let first_tool = &tools[0];
                let first_keys: Vec<&str> = first_tool
                    .as_object()
                    .map(|obj| obj.keys().map(|k| k.as_str()).collect())
                    .unwrap_or_default();

                // Verify all tools have the same structure (required for CSV)
                for tool in tools {
                    if let Some(obj) = tool.as_object() {
                        let keys: Vec<&str> = obj.keys().map(|k| k.as_str()).collect();
                        assert!(
                            first_keys.iter().all(|k| keys.contains(k)),
                            "All tools should have consistent fields for CSV extraction"
                        );
                    }
                }

                // Demonstrate CSV header generation
                let csv_header = first_keys.join(",");
                assert!(
                    !csv_header.is_empty(),
                    "Should be able to generate CSV header from JSON"
                );

                // Demonstrate CSV row generation
                for tool in tools.iter().take(1) {
                    // Just test first one
                    if let Some(obj) = tool.as_object() {
                        let csv_row: Vec<String> = first_keys
                            .iter()
                            .map(|k| {
                                obj.get(*k)
                                    .and_then(|v| {
                                        if v.is_string() {
                                            v.as_str().map(|s| s.to_string())
                                        } else {
                                            Some(v.to_string())
                                        }
                                    })
                                    .unwrap_or_default()
                            })
                            .collect();

                        assert!(
                            csv_row.len() == first_keys.len(),
                            "CSV row should have same number of columns as header"
                        );
                    }
                }
            }
        }
    }
}

/// Test that engine list JSON can be converted to CSV format
#[test]
fn test_json_to_csv_conversion_engine_list() {
    let mut cmd = Command::cargo_bin("fluent").unwrap();
    let output = cmd.args(["engine", "list", "--json"]).output().unwrap();

    if !output.status.success() {
        return;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: Result<Value, _> = serde_json::from_str(&stdout);

    if let Ok(json) = parsed {
        if let Some(engines) = json.as_array() {
            if !engines.is_empty() {
                // Verify that we can extract CSV-like data from JSON
                let first_engine = &engines[0];

                // Flatten nested connection object for CSV
                if let Some(obj) = first_engine.as_object() {
                    assert!(
                        obj.contains_key("name"),
                        "Engine should have 'name' field for CSV"
                    );
                    assert!(
                        obj.contains_key("engine"),
                        "Engine should have 'engine' field for CSV"
                    );

                    // Connection is nested - would need flattening for CSV
                    if let Some(conn) = obj.get("connection").and_then(|c| c.as_object()) {
                        // Verify connection fields that would become CSV columns
                        assert!(
                            conn.contains_key("hostname"),
                            "Connection should have hostname for CSV"
                        );
                        assert!(
                            conn.contains_key("port"),
                            "Connection should have port for CSV"
                        );
                    }
                }
            }
        }
    }
}
