use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn cli_help() {
    Command::cargo_bin("cli")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Minimal Memory CLI"));
}

#[test]
fn cli_add_and_query() {
    let path = "cli_test.jsonl";
    let _ = std::fs::remove_file(path);
    Command::cargo_bin("cli")
        .unwrap()
        .args([
            "--store", path, "add", "--actor", "tester", "--action", "run", "--target", "t",
        ])
        .assert()
        .success();
    let out = Command::cargo_bin("cli")
        .unwrap()
        .args(["--store", path, "query"])
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&out.stdout).contains("tester"));
    std::fs::remove_file(path).unwrap();
}

#[test]
fn cli_query_pagination() {
    let path = "cli_page.jsonl";
    let _ = std::fs::remove_file(path);
    for i in 0..5 {
        Command::cargo_bin("cli")
            .unwrap()
            .args([
                "--store",
                path,
                "add",
                "--actor",
                &format!("a{}", i),
                "--action",
                "run",
                "--target",
                "t",
            ])
            .assert()
            .success();
    }
    let out = Command::cargo_bin("cli")
        .unwrap()
        .args(["--store", path, "query", "--page", "2", "--page-size", "2"])
        .output()
        .unwrap();
    let out_str = String::from_utf8_lossy(&out.stdout);
    assert!(out_str.contains("a2"));
    assert!(!out_str.contains("actor: \"a0\""));
    std::fs::remove_file(path).unwrap();
}
