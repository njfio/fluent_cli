# Fluent Agent Unwrap() Fixes

## Priority Level: HIGH RISK

### 1. shell.rs:69 - unwrap_or(-1) on exit code
**Location**: `crates/fluent-agent/src/tools/shell.rs:69`
**Risk**: HIGH - Could mask important error states
**Context**: Converting exit code from Option<i32> to i32

#### Before:
```rust
exit_code: output.status.code().unwrap_or(-1),
```

#### After:
```rust
exit_code: output.status.code().unwrap_or_else(|| {
    // Different error codes for different failure scenarios
    if output.status.success() {
        0  // Success but no code (shouldn't happen)
    } else {
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            // Check if process was terminated by signal
            if let Some(signal) = output.status.signal() {
                return -signal; // Negative signal number
            }
        }
        -1 // Generic failure
    }
}),
```

### 2. filesystem.rs:47 - unwrap_or_default() on file_name
**Location**: `crates/fluent-agent/src/tools/filesystem.rs:47`
**Risk**: HIGH - Could create invalid paths
**Context**: Getting file name from path for non-existent files

#### Before:
```rust
canonical_parent.join(validated_path.file_name().unwrap_or_default())
```

#### After:
```rust
let file_name = validated_path
    .file_name()
    .ok_or_else(|| anyhow!("Path '{}' has no file name component", validated_path.display()))?;
canonical_parent.join(file_name)
```

### 3. filesystem.rs:278 - unwrap_or_default() on file_name in get_file_info
**Location**: `crates/fluent-agent/src/tools/filesystem.rs:278`
**Risk**: HIGH - Returns empty string for critical file info
**Context**: Getting file name for FileInfo struct

#### Before:
```rust
name: path
    .file_name()
    .unwrap_or_default()
    .to_string_lossy()
    .to_string(),
```

#### After:
```rust
name: path
    .file_name()
    .ok_or_else(|| anyhow!("Path '{}' has no file name component", path.display()))?
    .to_string_lossy()
    .to_string(),
```

## Priority Level: MEDIUM RISK (Test Code)

### 4. shell.rs:264 - expect() in test setup
**Location**: `crates/fluent-agent/src/tools/shell.rs:264`
**Risk**: MEDIUM - Test infrastructure failure
**Context**: Creating temporary directory for tests

#### Before:
```rust
let temp_dir = tempdir().expect("Failed to create temp directory");
```

#### After:
```rust
let temp_dir = tempdir().map_err(|e| {
    eprintln!("Test setup failed: Unable to create temp directory: {}", e);
    e
})?;
```

### 5. shell.rs:280 - expect() parsing command result
**Location**: `crates/fluent-agent/src/tools/shell.rs:280`
**Risk**: MEDIUM - Test assertion failure
**Context**: Parsing command execution result

#### Before:
```rust
.expect("Command execution failed");
```

#### After:
```rust
.map_err(|e| {
    eprintln!("Command execution failed: {}", e);
    e
})?;
```

### 6. shell.rs:283 - expect() parsing JSON
**Location**: `crates/fluent-agent/src/tools/shell.rs:283`
**Risk**: MEDIUM - Test validation failure
**Context**: Deserializing CommandResult

#### Before:
```rust
serde_json::from_str(&result).expect("Failed to parse command result");
```

#### After:
```rust
serde_json::from_str(&result).map_err(|e| {
    eprintln!("Failed to parse command result: {}\nResult was: {}", e, result);
    e
})?;
```

## Priority Level: LOW RISK (Test Code - Multiple Similar Patterns)

The following unwrap() calls in test code follow similar patterns and should be fixed uniformly:

### filesystem.rs Test unwraps:
- Line 390, 394-396, 409, 410, 438, 439, 447-455, 474, 475: Various `unwrap()` calls in test setup

#### Standard Test Pattern Fix:
Replace all test `unwrap()` calls with proper error propagation:

```rust
// Before:
let temp_dir = tempdir().unwrap();

// After:
let temp_dir = tempdir()?;
```

## Implementation Order

1. **Immediate (Production Code)**:
   - filesystem.rs:47 - Fix path construction for non-existent files
   - filesystem.rs:278 - Fix file name extraction in get_file_info
   - shell.rs:69 - Improve exit code handling

2. **Next Sprint (Test Improvements)**:
   - Update all test code to use Result<()> return type
   - Replace unwrap() with ? operator in tests
   - Add better error messages for test failures

## Additional Recommendations

1. **Add a project-wide lint**:
   ```toml
   # In Cargo.toml or .cargo/config.toml
   [lints.rust]
   unwrap_used = "warn"
   expect_used = "warn"
   ```

2. **Create helper functions** for common patterns:
   ```rust
   // For exit code handling
   fn extract_exit_code(status: &ExitStatus) -> i32 {
       // Implementation from fix #1
   }
   
   // For path validation
   fn validate_file_name(path: &Path) -> Result<&OsStr> {
       path.file_name()
           .ok_or_else(|| anyhow!("Path '{}' has no file name", path.display()))
   }
   ```

3. **Consider using derive_more or thiserror** for better error handling throughout the codebase.