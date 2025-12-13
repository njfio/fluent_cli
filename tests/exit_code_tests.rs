use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

/// Test that success cases exit with code 0
#[test]
fn exit_code_for_success() {
    let mut cmd = Command::cargo_bin("fluent").expect("binary");
    cmd.arg("--help");
    cmd.assert().success().code(predicate::eq(0));
}

/// Test that help/version requests exit successfully with code 0
#[test]
fn exit_code_for_version() {
    let mut cmd = Command::cargo_bin("fluent").expect("binary");
    cmd.arg("--version");
    cmd.assert().success().code(predicate::eq(0));
}

/// Test that invalid arguments return exit code 2 (USAGE_ERROR)
#[test]
fn exit_code_for_argparse_error() {
    let mut cmd = Command::cargo_bin("fluent").expect("binary");
    cmd.arg("not-a-real-command");
    cmd.assert().failure().code(predicate::eq(2));
}

/// Test that missing required arguments return exit code 2 (USAGE_ERROR)
#[test]
fn exit_code_for_missing_required_arg() {
    let mut cmd = Command::cargo_bin("fluent").expect("binary");
    cmd.args(["completions"]); // Missing --shell argument
    cmd.assert().failure().code(predicate::eq(2));
}

/// Test that missing pipeline file returns exit code 10 (CONFIG_ERROR)
#[test]
fn exit_code_for_missing_pipeline_file() {
    let mut cmd = Command::cargo_bin("fluent").expect("binary");
    cmd.args(["pipeline", "--file", "/definitely/missing.yaml"]);
    cmd.assert().failure().code(predicate::eq(10)); // Config error
}

/// Test that missing config file (when explicitly specified) returns exit code 10 (CONFIG_ERROR)
/// Note: Using "engine test" command which requires a config, unlike "engine list"
#[test]
fn exit_code_for_missing_config_file() {
    let mut cmd = Command::cargo_bin("fluent").expect("binary");
    cmd.args([
        "--config",
        "/definitely/missing.toml",
        "engine",
        "test",
        "some-engine",
    ]);
    cmd.assert().failure().code(predicate::eq(10)); // Config error
}

/// Test that nonexistent engine returns exit code 10 (CONFIG_ERROR)
#[test]
fn exit_code_for_engine_not_found() {
    let mut cmd = Command::cargo_bin("fluent").expect("binary");
    cmd.args(["engine", "test", "nonexistent-engine"]);
    cmd.assert().failure().code(predicate::eq(10)); // Config error
}

/// Test that commands that can run without config succeed
#[test]
fn exit_code_for_completions_success() {
    let mut cmd = Command::cargo_bin("fluent").expect("binary");
    cmd.args(["completions", "--shell", "bash"]);
    cmd.assert().success().code(predicate::eq(0));
}

/// Test that engine list can run without config
#[test]
fn exit_code_for_engine_list_no_config() {
    let mut cmd = Command::cargo_bin("fluent").expect("binary");
    cmd.args(["engine", "list"]);
    // This should succeed even without a config file
    cmd.assert().success().code(predicate::eq(0));
}

/// Test that tools list can run without config
#[test]
fn exit_code_for_tools_list_no_config() {
    let mut cmd = Command::cargo_bin("fluent").expect("binary");
    cmd.args(["tools", "list"]);
    // This should succeed even without a config file
    cmd.assert().success().code(predicate::eq(0));
}
