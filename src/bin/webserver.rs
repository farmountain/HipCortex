use hipcortex::memory_store::MemoryStore;
use hipcortex::web_server;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr: SocketAddr = "127.0.0.1:3030".parse()?;
    
    // Create memory store with File backend
    let memory_store = MemoryStore::new("memory.jsonl")?;
    let memory_store = Arc::new(Mutex::new(memory_store));
    
    println!("🚀 HipCortex REST API Server starting on http://{}", addr);
    println!("📚 Endpoints:");
    println!("  GET  /health - Health check");
    println!("  GET  /graph - Get symbolic graph");
    println!("  GET  /node/:id - Get symbolic node");
    println!("  POST /memory/add - Add memory record");
    println!("  GET  /memory/query - Query memory records");
    
    web_server::run_with_memory(addr, memory_store).await;
    Ok(())
}
