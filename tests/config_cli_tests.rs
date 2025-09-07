use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn config_schema_prints_json() {
    let mut cmd = Command::cargo_bin("fluent-config").expect("fluent-config binary");
    cmd.arg("schema");
    cmd.assert().success().stdout(predicate::str::starts_with("{")).stdout(predicate::str::contains("$schema").or(predicate::str::contains("schema")));
}
