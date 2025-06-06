use assert_cmd::Command;
use std::fs;

#[test]
fn cli_add_and_query() {
    let path = "cli_mem.jsonl";
    let _ = fs::remove_file(path);
    let mut cmd = Command::cargo_bin("hipcortex").unwrap();
    cmd.args(["--store", path, "add", "--actor", "u", "--action", "did", "--target", "t"])
        .assert()
        .success();

    let mut query = Command::cargo_bin("hipcortex").unwrap();
    query.args(["--store", path, "query", "--actor", "u"])
        .assert()
        .success();
    fs::remove_file(path).unwrap();
}
