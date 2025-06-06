use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn cli_help() {
    Command::cargo_bin("cli").unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Minimal Memory CLI"));
}

#[test]
fn cli_add_and_query() {
    let path = "cli_test.jsonl";
    let _ = std::fs::remove_file(path);
    Command::cargo_bin("cli").unwrap()
        .args(["--store", path, "add", "--actor", "tester", "--action", "run", "--target", "t"])
        .assert()
        .success();
    let out = Command::cargo_bin("cli").unwrap()
        .args(["--store", path, "query"])
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&out.stdout).contains("tester"));
    std::fs::remove_file(path).unwrap();
}
