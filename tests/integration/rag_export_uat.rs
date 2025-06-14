use hipcortex::knowledge_export::NotionExporter;
use hipcortex::rag_adapter::{LocalRagAdapter, RagAdapter};
use hipcortex::symbolic_store::SymbolicStore;
use mockito::{Matcher, Server};
use std::collections::HashMap;

#[test]
fn user_export_to_notion() {
    let mut store = SymbolicStore::new();
    store.add_node("Policy", HashMap::new());
    let adapter = LocalRagAdapter::new(&store);
    let results = adapter.retrieve("Policy").unwrap();
    let mut server = Server::new();
    let m = server
        .mock("POST", "/pages")
        .match_body(Matcher::Any)
        .with_status(200)
        .create();
    let exporter = NotionExporter::with_base_url("tok", &server.url());
    exporter.export_page("Policy", &results[0]).unwrap();
    m.assert();
}
