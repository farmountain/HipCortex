#[cfg(all(feature = "web-server", feature = "grpc-server"))]
use hipcortex::mcp_server::McpServer;
#[cfg(all(feature = "web-server", feature = "grpc-server"))]
use hipcortex::memory_store::MemoryStore;

#[cfg(all(feature = "web-server", feature = "grpc-server"))]
#[test]
fn mcp_server_init() {
    let path = "mcp_unit.jsonl";
    let _ = std::fs::remove_file(path);
    let store = MemoryStore::new(path).unwrap();
    let server = McpServer::new(store);
    assert_eq!(server.store().lock().unwrap().all().len(), 0);
    let graph = server.symbolic().lock().unwrap().export_graph();
    assert_eq!(graph.0.len(), 0);
    std::fs::remove_file(path).unwrap();
    std::fs::remove_file("mcp_unit.audit.log").unwrap();
}
