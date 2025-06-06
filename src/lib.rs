//! HipCortex library entry point.
//! Export all modules for easy external use.

#[path = "modules/temporal_indexer.rs"]
pub mod temporal_indexer;
#[path = "modules/procedural_cache.rs"]
pub mod procedural_cache;
pub mod memory_record;
pub mod memory_store;
pub mod memory_processor;
pub mod memory_query;
pub mod snapshot_manager;
pub mod memory_cli;
pub mod llm_clients;
#[path = "modules/symbolic_store.rs"]
pub mod symbolic_store;
#[path = "modules/perception_adapter.rs"]
pub mod perception_adapter;
#[path = "modules/integration_layer.rs"]
pub mod integration_layer;
#[path = "modules/aureus_bridge.rs"]
pub mod aureus_bridge;
pub mod vision_encoder;
pub mod memory;
pub mod schema;
#[cfg(feature = "web-server")]
pub mod web_server;
#[cfg(feature = "gui")]
pub mod gui;
