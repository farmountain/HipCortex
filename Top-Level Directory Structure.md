hipcortex/
├── src/
│   ├── lib.rs                  # Library entry (recommended for reuse)
│   ├── main.rs                 # CLI entry (optional, for demos/tests)
│   ├── temporal_indexer.rs
│   ├── procedural_cache.rs
│   ├── symbolic_store.rs
│   ├── perception_adapter.rs
│   ├── integration_layer.rs
│   ├── aureus_bridge.rs
├── tests/
│   └── integration_tests.rs
├── benches/
│   └── criterion_bench.rs
├── examples/
│   └── quickstart.rs           # Small runnable demo of core logic
├── docs/
│   ├── architecture.md
│   ├── usage.md
│   ├── integration.md
│   └── roadmap.md
├── .github/
│   ├── ISSUE_TEMPLATE.md
│   └── PULL_REQUEST_TEMPLATE.md
├── .vscode/
│   ├── settings.json
│   ├── launch.json
│   ├── extensions.json
│   └── tasks.json
├── Cargo.toml
├── Cargo.lock                  # (in .gitignore or tracked as needed)
├── .gitignore
├── README.md
├── LICENSE
