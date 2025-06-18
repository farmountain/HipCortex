#[cfg(all(feature = "web-server", feature = "grpc-server"))]
use hipcortex::mcp_server::McpServer;
#[cfg(all(feature = "web-server", feature = "grpc-server"))]
use hipcortex::memory_store::MemoryStore;
#[cfg(all(feature = "web-server", feature = "grpc-server"))]
use hipcortex::grpc_server::grpc::{
    memory_service_client::MemoryServiceClient, AddRecordRequest, ListRecordsRequest, MemoryRecord as ProtoRecord,
};
#[cfg(all(feature = "web-server", feature = "grpc-server"))]
use tokio::time::{sleep, Duration};
#[cfg(all(feature = "web-server", feature = "grpc-server"))]
use uuid::Uuid;

#[cfg(all(feature = "web-server", feature = "grpc-server"))]
#[tokio::test]
async fn mcp_server_handles_grpc_and_http() {
    let path = "mcp_sit.jsonl";
    let _ = std::fs::remove_file(path);
    let store = MemoryStore::new(path).unwrap();
    let server = McpServer::new(store);
    let http_addr: std::net::SocketAddr = "127.0.0.1:3141".parse().unwrap();
    let grpc_addr: std::net::SocketAddr = "127.0.0.1:5141".parse().unwrap();
    let srv = tokio::spawn(async move { server.serve(http_addr, grpc_addr).await.unwrap(); });
    sleep(Duration::from_millis(100)).await;

    let mut client = MemoryServiceClient::connect("http://127.0.0.1:5141").await.unwrap();
    let req = AddRecordRequest {
        record: Some(ProtoRecord {
            id: Uuid::new_v4().to_string(),
            record_type: "Symbolic".into(),
            timestamp: chrono::Utc::now().timestamp(),
            actor: "sit".into(),
            action: "run".into(),
            target: "t".into(),
            metadata: "{}".into(),
        }),
    };
    client.add_record(req).await.unwrap();
    let resp = client.list_records(ListRecordsRequest {}).await.unwrap().into_inner();
    assert_eq!(resp.records.len(), 1);

    let resp = reqwest::get("http://127.0.0.1:3141/health").await.unwrap();
    assert_eq!(resp.text().await.unwrap(), "ok");

    srv.abort();
    std::fs::remove_file(path).unwrap();
    std::fs::remove_file("mcp_sit.audit.log").unwrap();
}
