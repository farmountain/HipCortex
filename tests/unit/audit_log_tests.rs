use hipcortex::audit_log::AuditLog;

#[test]
fn append_and_verify() {
    let path = "test_audit.log";
    let _ = std::fs::remove_file(path);
    let mut log = AuditLog::new(path).unwrap();
    log.append("a", "act", "ok").unwrap();
    log.append("b", "act", "ok").unwrap();
    assert!(log.verify().unwrap());
    std::fs::remove_file(path).unwrap();
}
