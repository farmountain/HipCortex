#[cfg(feature = "plugin")]
use hipcortex::memory_record::{MemoryRecord, MemoryType};
#[cfg(feature = "plugin")]
use hipcortex::memory_store::MemoryStore;
use hipcortex::plugin_host::PluginHost;

#[test]
fn plugin_generates_memory() {
    let host = PluginHost::new();
    #[cfg(feature = "plugin")]
    {
        let path = "plugin_sit.jsonl";
        let _ = std::fs::remove_file(path);
        let mut store = MemoryStore::new(path).unwrap();
        let wat = "(module (func (export \"run\") (result i32) i32.const 5))";
        let bytes = wat::parse_str(wat).unwrap();
        let result = host.run_wasm(&bytes).unwrap();
        store
            .add(MemoryRecord::new(
                MemoryType::Procedural,
                "plugin".into(),
                "run".into(),
                result.to_string(),
                serde_json::json!({}),
            ))
            .unwrap();
        assert_eq!(store.all().len(), 1);
        std::fs::remove_file(path).unwrap();
    }
    #[cfg(not(feature = "plugin"))]
    {
        assert!(host.run_wasm(&[]).is_err());
    }
}
