#[cfg(all(feature = "web-server", feature = "grpc-server"))]
use crate::{
    aureus_bridge::AureusBridge,
    coherence::CoherenceChecker,
    grpc_server,
    integration_layer::IntegrationLayer,
    memory_store::MemoryStore,
    perception_adapter::PerceptionSession,
    persistence::MemoryBackend,
    procedural_cache::ProceduralCache,
    self_model::SelfModel,
    symbolic_store::{InMemoryGraph, SymbolicStore},
    temporal_indexer::TemporalIndexer,
    topological_memory::CausalTopoGraph,
    web_server,
    world_model_enhanced::WorldModelEnhanced,
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
    // held for rich agent defaults + live_beliefs wiring (conditional on HIPCORTEX_AGENT_DEFAULTS)
    self_model: Option<Arc<SelfModel>>,
    world_model: Option<Arc<WorldModelEnhanced>>,
    coherence: Option<Arc<CoherenceChecker>>,
    // Task 10: topo for defaults + loop exposure in mcp (surgical wiring)
    topo: Option<Arc<Mutex<CausalTopoGraph>>>,
}

#[cfg(all(feature = "web-server", feature = "grpc-server"))]
impl<B: MemoryBackend + Send + 'static> McpServer<B> {
    /// Create a new MCP server using the provided MemoryStore.
    pub fn new(store: MemoryStore<B>) -> Self {
        // default PerceptionSession with self/world/coherence for agent paths (mcp); gated by HIPCORTEX_AGENT_DEFAULTS (default off for conservative per spec)
        let sm = std::sync::Arc::new(SelfModel::new());
        let wm = std::sync::Arc::new(WorldModelEnhanced::new());
        let cc = std::sync::Arc::new(CoherenceChecker::new());
        // Task 10 wiring: create topo substrate and wire into PerceptionSession defaults (surgical like intel layer)
        let topo = Arc::new(Mutex::new(CausalTopoGraph::new()));
        // instantiate loop_engine (omega) for mcp defaults + exposure in auto paths
        let _loop_exposure = crate::loop_engine::LoopEngine::new(CausalTopoGraph::new());
        let session = if std::env::var("HIPCORTEX_AGENT_DEFAULTS").is_ok() {
            PerceptionSession::new()
                .with_self_model(sm.clone())
                .with_world_model(wm.clone())
                .with_coherence(cc.clone())
                .with_topo(topo.clone())
        } else {
            PerceptionSession::new()
        };
        Self {
            store: Arc::new(Mutex::new(store)),
            symbolic: Arc::new(Mutex::new(SymbolicStore::new())),
            indexer: Arc::new(Mutex::new(TemporalIndexer::new(256, 60))),
            fsm: Arc::new(Mutex::new(ProceduralCache::new())),
            bridge: Arc::new(Mutex::new(AureusBridge::new())),
            layer: IntegrationLayer::new().with_perception_session(session),
            self_model: Some(sm),
            world_model: Some(wm),
            coherence: Some(cc),
            topo: Some(topo),
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
    pub async fn serve(&self, http_addr: SocketAddr, grpc_addr: SocketAddr) -> anyhow::Result<()> {
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
