use assert_cmd::Command;

#[test]
fn cli_cache_roundtrip() {
    let path = "cache_cli.json";
    let store = "dummy.jsonl";
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(store);
    Command::cargo_bin("cli")
        .unwrap()
        .args([
            "--store",
            store,
            "cache-put",
            "--cache",
            path,
            "foo",
            "--embedding",
            "1.0,0.0",
        ])
        .assert()
        .success();
    let out = Command::cargo_bin("cli")
        .unwrap()
        .args([
            "--store",
            store,
            "cache-get",
            "--cache",
            path,
            "--query",
            "1.0,0.0",
        ])
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&out.stdout).contains("foo"));
    std::fs::remove_file(path).unwrap();
    let _ = std::fs::remove_file(store);
}
