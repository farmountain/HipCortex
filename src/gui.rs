#[cfg(feature = "gui")]
use serde::Serialize;
#[cfg(feature = "gui")]
use std::sync::Mutex;
#[cfg(feature = "gui")]
use tauri::State;

#[cfg(feature = "gui")]
use crate::{
    aureus_bridge::AureusBridge,
    memory_record::{MemoryRecord, MemoryType},
    memory_store::MemoryStore,
    perception_adapter::{Modality, PerceptInput, PerceptionAdapter},
    procedural_cache::ProceduralCache,
    symbolic_store::SymbolicStore,
    temporal_indexer::TemporalIndexer,
};

#[cfg(feature = "gui")]
pub struct GuiState {
    symbolic: Mutex<SymbolicStore>,
    temporal: Mutex<TemporalIndexer<String>>,
    procedural: Mutex<ProceduralCache>,
    aureus: Mutex<AureusBridge>,
    memory: Mutex<MemoryStore>,
}

#[cfg(feature = "gui")]
impl GuiState {
    pub fn new() -> Self {
        Self {
            symbolic: Mutex::new(SymbolicStore::new()),
            temporal: Mutex::new(TemporalIndexer::new(100, 3600)),
            procedural: Mutex::new(ProceduralCache::new()),
            aureus: Mutex::new(AureusBridge::new()),
            memory: Mutex::new(MemoryStore::new("memory_gui.jsonl").unwrap()),
        }
    }
}

#[cfg(feature = "gui")]
#[derive(Serialize)]
struct NodeDto {
    id: String,
    label: String,
}

#[cfg(feature = "gui")]
#[derive(Serialize)]
struct EdgeDto {
    from: String,
    to: String,
    relation: String,
}

#[cfg(feature = "gui")]
#[tauri::command]
fn get_symbolic_graph(state: State<GuiState>) -> (Vec<NodeDto>, Vec<EdgeDto>) {
    let store = state.symbolic.lock().unwrap();
    let nodes = store
        .nodes
        .values()
        .map(|n| NodeDto {
            id: n.id.to_string(),
            label: n.label.clone(),
        })
        .collect();
    let edges = store
        .edges
        .iter()
        .map(|e| EdgeDto {
            from: e.from.to_string(),
            to: e.to.to_string(),
            relation: e.relation.clone(),
        })
        .collect();
    (nodes, edges)
}

#[cfg(feature = "gui")]
#[tauri::command]
fn run_reflexion(state: State<GuiState>) -> usize {
    let mut bridge = state.aureus.lock().unwrap();
    bridge.reflexion_loop();
    bridge.loops_run()
}

#[cfg(feature = "gui")]
#[tauri::command]
fn send_perception(state: State<GuiState>, text: String) -> String {
    let input = PerceptInput {
        modality: Modality::Text,
        text: Some(text.clone()),
        embedding: None,
        image_data: None,
        tags: vec![],
    };
    PerceptionAdapter::adapt(input);
    let mut mem = state.memory.lock().unwrap();
    let rec = MemoryRecord::new(
        MemoryType::Perception,
        "user".into(),
        "input".into(),
        text.clone(),
        serde_json::json!({}),
    );
    let _ = mem.add(rec);
    format!("perceived: {}", text)
}

#[cfg(feature = "gui")]
#[tauri::command]
fn cli_command(state: State<GuiState>, cmd: String) -> String {
    match cmd.as_str() {
        "trace list" => {
            let mem = state.memory.lock().unwrap();
            format!("{} records", mem.all().len())
        }
        _ => format!("unknown command: {}", cmd),
    }
}

#[cfg(feature = "gui")]
pub fn launch() -> tauri::Result<()> {
    tauri::Builder::default()
        .manage(GuiState::new())
        .invoke_handler(tauri::generate_handler![
            get_symbolic_graph,
            run_reflexion,
            send_perception,
            cli_command
        ])
        .run(tauri::generate_context!())
}
