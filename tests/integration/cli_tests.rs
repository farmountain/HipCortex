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
#[test]
fn cli_snapshot_and_restore() {
    let path = "cli_snap.jsonl";
    let tag = "snap";
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(std::path::Path::new(path).with_extension(format!("{}.tar.gz", tag)));
    // add initial record
    Command::cargo_bin("cli")
        .unwrap()
        .args(["--store", path, "add", "--actor", "tester", "--action", "remember", "--target", "orig"])
        .assert()
        .success();
    // snapshot
    Command::cargo_bin("cli")
        .unwrap()
        .args(["--store", path, "snapshot", tag])
        .assert()
        .success();
    // add extra record
    Command::cargo_bin("cli")
        .unwrap()
        .args(["--store", path, "add", "--actor", "tester", "--action", "remember", "--target", "rolled back"])
        .assert()
        .success();
    // restore
    Command::cargo_bin("cli")
        .unwrap()
        .args(["--store", path, "restore", tag])
        .assert()
        .success();
    // query for rolled back text should be empty
    let out = Command::cargo_bin("cli")
        .unwrap()
        .args(["--store", path, "query", "--query", "rolled back"])
        .output()
        .unwrap();
    assert!(!String::from_utf8_lossy(&out.stdout).contains("rolled back"));
    std::fs::remove_file(path).unwrap();
    std::fs::remove_file(std::path::Path::new(path).with_extension(format!("{}.tar.gz", tag))).unwrap();
}

#[test]
fn cli_query_filter_actor() {
    let path = "cli_filter.jsonl";
    let _ = std::fs::remove_file(path);
    // two different actors
    Command::cargo_bin("cli")
        .unwrap()
        .args(["--store", path, "add", "--actor", "alice", "--action", "say", "--target", "hi"])
        .assert()
        .success();
    Command::cargo_bin("cli")
        .unwrap()
        .args(["--store", path, "add", "--actor", "bob", "--action", "say", "--target", "bye"])
        .assert()
        .success();
    let out = Command::cargo_bin("cli")
        .unwrap()
        .args(["--store", path, "query", "--actor", "alice"])
        .output()
        .unwrap();
    let out_str = String::from_utf8_lossy(&out.stdout);
    assert!(out_str.contains("alice"));
    assert!(!out_str.contains("bob"));
    std::fs::remove_file(path).unwrap();
}

#[test]
fn cli_prompt_stores_reflexion() {
    let path = "cli_prompt.jsonl";
    let _ = std::fs::remove_file(path);
    // run prompt with dummy api key so network errors are ignored
    Command::cargo_bin("cli")
        .unwrap()
        .env("OPENAI_API_KEY", "test")
        .args(["--store", path, "prompt", "hello?"])
        .assert()
        .success();
    let data = std::fs::read_to_string(path).unwrap();
    assert!(data.contains("\"action\":\"prompt\""));
    std::fs::remove_file(path).unwrap();
}

