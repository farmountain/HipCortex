#[cfg(all(feature = "web-server", feature = "grpc-server"))]
use hipcortex::mcp_server::McpServer;
#[cfg(all(feature = "web-server", feature = "grpc-server"))]
use hipcortex::memory_store::MemoryStore;
#[cfg(all(feature = "web-server", feature = "grpc-server"))]
use std::net::SocketAddr;

#[cfg(all(feature = "web-server", feature = "grpc-server"))]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let store = MemoryStore::new("memory.jsonl")?;
    let server = McpServer::new(store);
    let http: SocketAddr = "127.0.0.1:8080".parse()?;
    let grpc: SocketAddr = "127.0.0.1:50051".parse()?;
    server.serve(http, grpc).await?;
    Ok(())
}

#[cfg(not(all(feature = "web-server", feature = "grpc-server")))]
fn main() {
    eprintln!("This example requires the web-server and grpc-server features");
}
