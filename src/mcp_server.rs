#[cfg(all(feature = "web-server", feature = "grpc-server"))]
use crate::{
    aureus_bridge::AureusBridge,
    grpc_server,
    integration_layer::IntegrationLayer,
    memory_store::MemoryStore,
    persistence::MemoryBackend,
    procedural_cache::ProceduralCache,
    symbolic_store::{InMemoryGraph, SymbolicStore},
    temporal_indexer::TemporalIndexer,
    web_server,
};
#[cfg(all(feature = "web-server", feature = "grpc-server"))]
use std::net::SocketAddr;
#[cfg(all(feature = "web-server", feature = "grpc-server"))]
use std::sync::{Arc, Mutex};
#[cfg(all(feature = "web-server", feature = "grpc-server"))]
use uuid::Uuid;

#[cfg(all(feature = "web-server", feature = "grpc-server"))]
/// Combined MCP server struct holding the core modules and stores.
pub struct McpServer<B: MemoryBackend + Send + 'static> {
    store: Arc<Mutex<MemoryStore<B>>>,
    symbolic: Arc<Mutex<SymbolicStore<InMemoryGraph>>>,
    indexer: Arc<Mutex<TemporalIndexer<Uuid>>>,
    fsm: Arc<Mutex<ProceduralCache>>,
    bridge: Arc<Mutex<AureusBridge>>,
    pub layer: IntegrationLayer,
}

#[cfg(all(feature = "web-server", feature = "grpc-server"))]
impl<B: MemoryBackend + Send + 'static> McpServer<B> {
    /// Create a new MCP server using the provided MemoryStore.
    pub fn new(store: MemoryStore<B>) -> Self {
        Self {
            store: Arc::new(Mutex::new(store)),
            symbolic: Arc::new(Mutex::new(SymbolicStore::new())),
            indexer: Arc::new(Mutex::new(TemporalIndexer::new(256, 60))),
            fsm: Arc::new(Mutex::new(ProceduralCache::new())),
            bridge: Arc::new(Mutex::new(AureusBridge::new())),
            layer: IntegrationLayer::new(),
        }
    }

    /// Access the underlying memory store.
    pub fn store(&self) -> Arc<Mutex<MemoryStore<B>>> {
        self.store.clone()
    }

    /// Access the symbolic store for custom logic.
    pub fn symbolic(&self) -> Arc<Mutex<SymbolicStore<InMemoryGraph>>> {
        self.symbolic.clone()
    }

    /// Start both the REST and gRPC servers concurrently.
    pub async fn serve(
        &self,
        http_addr: SocketAddr,
        grpc_addr: SocketAddr,
    ) -> anyhow::Result<()> {
        let store = self.store.clone();
        let grpc = tokio::spawn(async move {
            grpc_server::serve(grpc_addr, store).await.unwrap();
        });

        let symbolic = self.symbolic.clone();
        let http = tokio::spawn(async move {
            web_server::run_with_store(http_addr, symbolic).await;
        });

        grpc.await?;
        http.await.unwrap();
        Ok(())
    }
}
