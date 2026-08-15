use hipcortex::mcp_bridge;
use hipcortex::openmanus_bridge;

#[test]
fn openmanus_to_percept() {
    let json = r#"{"role":"user","content":"hello"}"#;
    let percept = openmanus_bridge::decode_request(json).unwrap();
    assert_eq!(percept.text.unwrap(), "hello");
    assert_eq!(percept.tags[0], "user");
}

#[test]
fn mcp_to_percept() {
    let json = r#"{"agent":"a1","message":"ping"}"#;
    let percept = mcp_bridge::decode_request(json).unwrap();
    assert_eq!(percept.text.unwrap(), "ping");
    assert_eq!(percept.tags[0], "a1");
}
