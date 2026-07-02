use hipcortex::aureus_bridge::AureusBridge;
use hipcortex::coherence::CoherenceChecker;
use hipcortex::memory_store::MemoryStore;
use hipcortex::self_model::{SelfModel, CapabilityDescriptor};
use hipcortex::symbolic_store::{InMemoryGraph, SymbolicStore};
use hipcortex::web_server::{self, AppState};
use hipcortex::world_model_enhanced::WorldModelEnhanced;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, RwLock};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3030);
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse()?;
    let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| ".".to_string());

    // ── Memory store ─────────────────────────────────────────────────────────
    let store_path = format!("{}/memory.jsonl", data_dir);
    let memory_store = Arc::new(Mutex::new(MemoryStore::new(&store_path)?));

    // ── WorldModelEnhanced: load from disk or start fresh ────────────────────
    let wm_path = format!("{}/worldmodel.json", data_dir);
    let world_model = match WorldModelEnhanced::load(&wm_path) {
        Ok(wm) => {
            println!("WorldModel: loaded {} transitions from {}", wm.transition_count(), wm_path);
            wm
        }
        Err(_) => {
            println!("WorldModel: starting fresh (no {} found)", wm_path);
            WorldModelEnhanced::new()
        }
    };
    let world_model = Arc::new(RwLock::new(world_model));

    // ── SelfModel: bootstrap with registered capabilities ────────────────────
    let self_model = Arc::new(SelfModel::new());
    for op in &["add_memory", "search_memory", "query_memory", "ingest",
                "bulk_add", "forget", "reflect", "context"] {
        self_model.register_capability(CapabilityDescriptor {
            name: op.to_string(),
            description: format!("HipCortex {} operation", op),
            required_cpu_percent: 5.0,
            required_memory_mb: 50.0,
            limitations: vec![],
        }).ok();
    }

    // ── Assemble AppState ────────────────────────────────────────────────────
    let state = AppState {
        memory_store: memory_store.clone(),
        symbolic_store: Arc::new(Mutex::new(SymbolicStore::<InMemoryGraph>::new())),
        world_model: world_model.clone(),
        aureus: Arc::new(Mutex::new(AureusBridge::new())),
        self_model,
        coherence: Arc::new(CoherenceChecker::new()),
        topo_graph: Arc::new(Mutex::new(hipcortex::topological_memory::CausalTopoGraph::new())),
    };

    // ── Periodic WorldModel flush every 5 minutes ────────────────────────────
    let wm_flush = world_model.clone();
    let wm_path_flush = wm_path.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300));
        loop {
            interval.tick().await;
            if let Ok(wm) = wm_flush.read() {
                if let Err(e) = wm.save(&wm_path_flush) {
                    eprintln!("[WorldModel] periodic flush error: {}", e);
                }
            }
        }
    });

    // ── Print startup info ───────────────────────────────────────────────────
    println!("HipCortex REST API  |  listening on http://{}", addr);
    let keys_configured = std::env::var("HIPCORTEX_API_KEYS").is_ok();
    if keys_configured {
        println!("Auth: API key required (X-Api-Key header)");
    } else {
        println!("Auth: open (set HIPCORTEX_API_KEYS to enable)");
    }

    // ── Graceful shutdown ────────────────────────────────────────────────────
    let store_for_signal = memory_store.clone();
    let wm_for_signal = world_model.clone();
    let wm_path_signal = wm_path.clone();

    tokio::select! {
        _ = web_server::run_with_state(addr, state) => {
            println!("Server exited normally.");
        }
        _ = tokio::signal::ctrl_c() => {
            println!("\nShutdown signal received — flushing stores...");
            if let Ok(mut ms) = store_for_signal.lock() {
                match ms.flush() {
                    Ok(_)  => println!("MemoryStore flushed."),
                    Err(e) => eprintln!("MemoryStore flush error: {}", e),
                }
            }
            if let Ok(wm) = wm_for_signal.read() {
                match wm.save(&wm_path_signal) {
                    Ok(_)  => println!("WorldModel saved to {}.", wm_path_signal),
                    Err(e) => eprintln!("WorldModel save error: {}", e),
                }
            }
        }
    }
    Ok(())
}
