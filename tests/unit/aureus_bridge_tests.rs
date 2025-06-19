use hipcortex::aureus_bridge::AureusBridge;
use hipcortex::hypotheses_graph::HypothesesGraph;
use hipcortex::llm_clients::mock::MockClient;
use hipcortex::memory_store::MemoryStore;

struct SeqClient {
    responses: Vec<String>,
    idx: std::sync::Mutex<usize>,
}

impl SeqClient {
    fn new(responses: Vec<String>) -> Self {
        Self {
            responses,
            idx: std::sync::Mutex::new(0),
        }
    }
}

impl hipcortex::llm_clients::LLMClient for SeqClient {
    fn generate_response(&self, _prompt: &str) -> String {
        let mut i = self.idx.lock().unwrap();
        let resp = self.responses[*i].clone();
        if *i + 1 < self.responses.len() {
            *i += 1;
        }
        resp
    }
}

#[test]
fn bayesian_update() {
    let bridge = AureusBridge::new();
    let post = bridge.update_belief(0.6, 0.8);
    let expected = (0.6 * 0.8) / ((0.6 * 0.8) + ((1.0 - 0.6) * (1.0 - 0.8)));
    approx::assert_abs_diff_eq!(post, expected, epsilon = 1e-6);
}

#[test]
fn graph_no_cycles() {
    let mut graph = HypothesesGraph::new();
    use hipcortex::aureus_bridge::ReflexionHypothesis;
    let a = graph.add_hypothesis(
        ReflexionHypothesis {
            text: "A".into(),
            confidence: 0.5,
            evidence: vec![],
        },
        None,
        true,
    );
    graph.add_hypothesis(
        ReflexionHypothesis {
            text: "B".into(),
            confidence: 0.5,
            evidence: vec![],
        },
        Some(a),
        true,
    );
    assert!(!graph.has_cycles());
}

#[test]
fn monte_carlo_selects_best() {
    let responses: Vec<String> = vec![
        "HypA\nConfidence: 0.4".to_string(),
        "HypB\nConfidence: 0.8".to_string(),
        "HypB\nConfidence: 0.9".to_string(),
    ];
    let client = SeqClient::new(responses);
    let mut bridge = AureusBridge::with_client(Box::new(client));
    let path = "mc_test.jsonl";
    let _ = std::fs::remove_file(path);
    let mut store = MemoryStore::new(path).unwrap();
    store.clear();
    let best = bridge.run_monte_carlo("ctx", &mut store, 3);
    assert_eq!(best.text, "HypB");
    assert!(best.confidence > 0.8);
    std::fs::remove_file(path).ok();
}

#[test]
fn reflexion_record_contains_confidence() {
    let mut bridge = AureusBridge::with_client(Box::new(MockClient));
    let path = "reflex.jsonl";
    let _ = std::fs::remove_file(path);
    let mut store = MemoryStore::new(path).unwrap();
    store.clear();
    bridge.reflexion_loop("ctx", &mut store);
    let recs = store.all();
    assert_eq!(recs.len(), 1);
    assert!(recs[0].metadata.get("confidence").is_some());
    std::fs::remove_file(path).ok();
}
