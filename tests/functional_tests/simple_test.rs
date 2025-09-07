//! Simple test to verify the test setup works

use anyhow::Result;
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_fluent_help() -> Result<()> {
    let mut cmd = Command::cargo_bin("fluent")?;
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("A powerful CLI for interacting with various AI engines"));
    Ok(())
}

#[test]
fn test_fluent_version() -> Result<()> {
    let mut cmd = Command::cargo_bin("fluent")?;
    cmd.arg("--version");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("fluent 0.1.0"));
    Ok(())
}