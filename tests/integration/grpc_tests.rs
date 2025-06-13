#[cfg(feature = "grpc-server")]
use hipcortex::grpc_server::grpc::{
    memory_service_client::MemoryServiceClient, AddRecordRequest, ListRecordsRequest,
    MemoryRecord as ProtoRecord,
};
#[cfg(feature = "grpc-server")]
use hipcortex::grpc_server::serve;
#[cfg(feature = "grpc-server")]
use hipcortex::memory_store::MemoryStore;
#[cfg(feature = "grpc-server")]
use std::sync::{Arc, Mutex};
#[cfg(feature = "grpc-server")]
use tokio::time::{sleep, Duration};

#[cfg(feature = "grpc-server")]
#[tokio::test]
async fn grpc_add_and_list() {
    let path = "grpc_test.jsonl";
    let _ = std::fs::remove_file(path);
    let store = MemoryStore::new(path).unwrap();
    let store = Arc::new(Mutex::new(store));
    let addr: std::net::SocketAddr = "127.0.0.1:50051".parse().unwrap();
    let srv_store = store.clone();
    let srv = tokio::spawn(async move {
        serve(addr, srv_store).await.unwrap();
    });
    // give server time to start
    sleep(Duration::from_millis(100)).await;
    let mut client = MemoryServiceClient::connect("http://127.0.0.1:50051")
        .await
        .unwrap();
    let req = AddRecordRequest {
        record: Some(ProtoRecord {
            id: uuid::Uuid::new_v4().to_string(),
            record_type: "Symbolic".into(),
            timestamp: chrono::Utc::now().timestamp(),
            actor: "tester".into(),
            action: "run".into(),
            target: "t".into(),
            metadata: "{}".into(),
        }),
    };
    client.add_record(req).await.unwrap();
    let resp = client
        .list_records(ListRecordsRequest {})
        .await
        .unwrap()
        .into_inner();
    assert_eq!(resp.records.len(), 1);
    srv.abort();
    std::fs::remove_file(path).unwrap();
}
