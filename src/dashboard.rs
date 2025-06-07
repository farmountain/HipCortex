#[cfg(feature = "web-server")]
use crate::memory_store::MemoryStore;
#[cfg(feature = "web-server")]
use crate::persistence::MemoryBackend;
#[cfg(feature = "web-server")]
use axum::{routing::get, Json, Router};

#[cfg(feature = "web-server")]
pub fn routes<B: MemoryBackend + 'static>(
    store: std::sync::Arc<std::sync::Mutex<MemoryStore<B>>>,
) -> Router {
    Router::new().route(
        "/nodes",
        get(move || {
            let store = store.lock().unwrap();
            let data: Vec<_> = store.all().iter().map(|r| &r.target).cloned().collect();
            async move { Json(data) }
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory_record::{MemoryRecord, MemoryType};
    use crate::memory_store::MemoryStore;

    #[cfg(feature = "web-server")]
    #[tokio::test]
    async fn router_build() {
        let path = "dash.jsonl";
        let _ = std::fs::remove_file(path);
        let store = MemoryStore::new(path).unwrap();
        let arc = std::sync::Arc::new(std::sync::Mutex::new(store));
        let _app = routes(arc);
        std::fs::remove_file(path).unwrap();
    }
}
