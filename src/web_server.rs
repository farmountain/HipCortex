#[cfg(feature = "web-server")]
use axum::{Router, routing::get};
#[cfg(feature = "web-server")]
use std::net::SocketAddr;

#[cfg(feature = "web-server")]
pub async fn run(addr: SocketAddr) {
    let app = Router::new().route("/health", get(|| async { "ok" }));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .expect("server failed");
}
