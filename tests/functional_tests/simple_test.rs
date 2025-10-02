//! Simple test to verify the test setup works

use anyhow::Result;
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_fluent_help() -> Result<()> {
    let mut cmd = Command::cargo_bin("fluent")?;
    cmd.arg("--help");
    cmd.assert().success().stdout(predicate::str::contains(
        "A powerful CLI for interacting with various AI engines",
    ));
    Ok(())
}

#[test]
fn test_fluent_version() -> Result<()> {
    let mut cmd = Command::cargo_bin("fluent")?;
    let mut cmd = Command::cargo_bin("fluent")?;
    cmd.arg("--version");
    let expected = format!("fluent {}", env!("CARGO_PKG_VERSION"));
    cmd.assert()
        .success()
        .stdout(predicate::str::contains(expected));
    Ok(())
}
