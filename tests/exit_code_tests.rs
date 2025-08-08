use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

#[test]
fn exit_code_for_argparse_error() {
    let mut cmd = Command::cargo_bin("fluent").expect("binary");
    cmd.arg("not-a-real-command");
    cmd.assert().failure().code(predicate::eq(2));
}

#[test]
fn exit_code_for_missing_pipeline_file() {
    let mut cmd = Command::cargo_bin("fluent").expect("binary");
    cmd.args(["pipeline", "--file", "/definitely/missing.yaml", "--input", "hi"]);
    cmd.assert().failure().code(predicate::eq(10)); // Config error
}

#[test]
fn exit_code_for_engine_not_found() {
    let mut cmd = Command::cargo_bin("fluent").expect("binary");
    cmd.args(["engine", "test", "--engine", "nonexistent-engine"]);
    cmd.assert().failure().code(predicate::eq(10)); // Config error
}
