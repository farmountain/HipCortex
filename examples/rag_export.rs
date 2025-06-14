use hipcortex::knowledge_export::PdfExporter;
use hipcortex::rag_adapter::{LocalRagAdapter, RagAdapter};
use hipcortex::symbolic_store::SymbolicStore;
use std::collections::HashMap;

fn main() {
    let mut store = SymbolicStore::new();
    store.add_node("ExampleDoc", HashMap::new());
    let adapter = LocalRagAdapter::new(&store);
    let results = adapter.retrieve("Example").unwrap();
    PdfExporter::export_to_file("output.pdf", &results[0]).unwrap();
    println!("Exported to output.pdf");
}
