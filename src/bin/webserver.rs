use hipcortex::memory_store::MemoryStore;
use hipcortex::web_server;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // PORT env var: Fly.io sets this automatically; default 3030 for local dev
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3030);

    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse()?;

    let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| ".".to_string());
    let store_path = format!("{}/memory.jsonl", data_dir);

    let memory_store = MemoryStore::new(&store_path)?;
    let memory_store = Arc::new(Mutex::new(memory_store));

    println!("HipCortex REST API  |  listening on http://{}", addr);

    let keys_configured = std::env::var("HIPCORTEX_API_KEYS").is_ok();
    if keys_configured {
        println!("Auth: API key required (X-Api-Key header)");
    } else {
        println!("Auth: open (set HIPCORTEX_API_KEYS to enable)");
    }

    println!();
    println!("Endpoints:");
    println!("  GET    /health                 health check");
    println!("  GET    /graph                  symbolic graph (nodes + edges)");
    println!("  GET    /node/:id               symbolic node by UUID");
    println!("  POST   /memory/add             store memory record");
    println!("  GET    /memory/query           query records (actor/action/type/limit)");
    println!("  DELETE /memory/forget/:actor   GDPR right-to-forget");
    println!("  GET    /coherence/status       coherence metrics");
    println!("  POST   /memory/search          semantic + keyword search");
    println!("  GET    /stats                  live server statistics");
    println!("  GET    /tier                   API key tier info");

    web_server::run_with_memory(addr, memory_store).await;
    Ok(())
}
