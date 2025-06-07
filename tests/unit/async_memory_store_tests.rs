#[cfg(feature = "async-store")]
use hipcortex::async_memory_store::AsyncMemoryStore;
#[cfg(feature = "async-store")]
use hipcortex::memory_record::{MemoryRecord, MemoryType};
#[cfg(feature = "async-store")]
use hipcortex::persistence::AsyncFileBackend;

#[cfg(feature = "async-store")]
#[tokio::test]
async fn async_store_add() {
    let path = "async_store.jsonl";
    let _ = tokio::fs::remove_file(path).await;
    let backend = AsyncFileBackend::new(path, false).await.unwrap();
    let mut store = AsyncMemoryStore::new(backend, std::path::Path::new("async_store.audit"), 2)
        .await
        .unwrap();
    let r = MemoryRecord::new(
        MemoryType::Symbolic,
        "u".into(),
        "a".into(),
        "b".into(),
        serde_json::json!({}),
    );
    store.add(r).await.unwrap();
    store.flush().await.unwrap();
    assert_eq!(store.all().len(), 1);
    tokio::fs::remove_file(path).await.unwrap();
}

#[cfg(feature = "async-store")]
#[tokio::test]
async fn async_store_encrypted() {
    let path = "async_store_enc.jsonl";
    let _ = tokio::fs::remove_file(path).await;
    let key = [9u8; 32];
    let backend = AsyncFileBackend::new_encrypted(path, key).await.unwrap();
    let mut store = AsyncMemoryStore::new(backend, std::path::Path::new("async_enc.audit"), 1)
        .await
        .unwrap();
    store
        .add(MemoryRecord::new(
            MemoryType::Symbolic,
            "x".into(),
            "a".into(),
            "b".into(),
            serde_json::json!({}),
        ))
        .await
        .unwrap();
    store.flush().await.unwrap();
    drop(store);
    let backend = AsyncFileBackend::new_encrypted(path, key).await.unwrap();
    let store = AsyncMemoryStore::new(backend, std::path::Path::new("async_enc.audit"), 1)
        .await
        .unwrap();
    assert_eq!(store.all().len(), 1);
    tokio::fs::remove_file(path).await.unwrap();
    tokio::fs::remove_file("async_enc.audit").await.unwrap();
}

#[cfg(feature = "async-store")]
#[tokio::test]
async fn async_store_wal_recovery() {
    let path = "async_store_wal.jsonl";
    let _ = tokio::fs::remove_file(path).await;
    let _ = tokio::fs::remove_file("async_store_wal.wal").await;
    let backend = AsyncFileBackend::new(path, false).await.unwrap();
    let mut store = AsyncMemoryStore::new(backend, std::path::Path::new("async_wal.audit"), 8)
        .await
        .unwrap();
    store
        .add(MemoryRecord::new(
            MemoryType::Symbolic,
            "x".into(),
            "y".into(),
            "z".into(),
            serde_json::json!({}),
        ))
        .await
        .unwrap();
    store.flush().await.unwrap();
    drop(store);
    let backend = AsyncFileBackend::new(path, false).await.unwrap();
    let store = AsyncMemoryStore::new(backend, std::path::Path::new("async_wal.audit"), 8)
        .await
        .unwrap();
    assert_eq!(store.all().len(), 1);
    let _ = tokio::fs::remove_file(path).await;
    let _ = tokio::fs::remove_file("async_store_wal.wal").await;
    let _ = tokio::fs::remove_file("async_wal.audit").await;
}
