use hipcortex::knowledge_export::PdfExporter;
use hipcortex::rag_adapter::{LocalRagAdapter, RagAdapter};
use hipcortex::symbolic_store::SymbolicStore;
use std::collections::HashMap;

#[test]
fn rag_to_pdf_round_trip() {
    let mut store = SymbolicStore::new();
    store.add_node("Guide", HashMap::new());
    let adapter = LocalRagAdapter::new(&store);
    let results = adapter.retrieve("Guide").unwrap();
    let tmp = tempfile::NamedTempFile::new().unwrap();
    PdfExporter::export_to_file(tmp.path().to_str().unwrap(), &results[0]).unwrap();
    assert!(tmp.path().exists());
}
