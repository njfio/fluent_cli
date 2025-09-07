use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn tools_list_json_succeeds() {
    let mut cmd = Command::cargo_bin("fluent").expect("fluent binary");
    cmd.args(["tools", "list", "--json"]);
    cmd.assert().success().stdout(predicate::str::starts_with("[").or(predicate::str::starts_with("{")));
}

#[test]
fn engine_list_json_succeeds() {
    let mut cmd = Command::cargo_bin("fluent").expect("fluent binary");
    cmd.args(["engine", "list", "--json"]);
    // Allow empty arrays or objects
    cmd.assert().success().stdout(predicate::str::contains("[").or(predicate::str::contains("{")));
}
