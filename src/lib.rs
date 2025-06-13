//! HipCortex library entry point.
//! Export all modules for easy external use.

pub mod a2a_protocol;
#[cfg(feature = "async-store")]
pub mod async_memory_store;
pub mod audit_log;
#[path = "modules/aureus_bridge.rs"]
pub mod aureus_bridge;
#[cfg(feature = "web-server")]
pub mod dashboard;
#[cfg(feature = "gui")]
pub mod gui;
#[path = "modules/integration_layer.rs"]
pub mod integration_layer;
pub mod llm_clients;
pub mod memory;
pub mod conversation_memory;
pub mod memory_cli;
pub mod memory_diff;
pub mod memory_processor;
pub mod memory_query;
pub mod memory_record;
pub mod memory_store;
#[path = "modules/perception_adapter.rs"]
pub mod perception_adapter;
pub mod persistence;
pub mod plugin_host;
#[path = "modules/procedural_cache.rs"]
pub mod procedural_cache;
#[cfg(feature = "rocksdb-backend")]
pub mod rocksdb_backend;
pub mod sandbox;
pub mod schema;
pub mod segmented_buffer;
pub mod semantic_compression;
pub mod snapshot_manager;
#[path = "modules/symbolic_store.rs"]
pub mod symbolic_store;
#[path = "modules/temporal_indexer.rs"]
pub mod temporal_indexer;
#[cfg(feature = "async-store")]
pub use persistence::{AsyncFileBackend, AsyncMemoryBackend};
#[cfg(feature = "grpc-server")]
pub mod grpc_server;
pub mod vision_encoder;
#[cfg(feature = "web-server")]
pub mod web_server;
