import json, os

with open('D:/all_projects/hipcortex/.ua/intermediate/assembled-graph.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

nodes = data.get('nodes', [])
all_ids = [n['id'] for n in nodes]

layers = {
    "layer:core-memory-engine": {
        "name": "Core Memory Engine",
        "description": "Canonical MemoryRecord data model, MemoryStore persistence (JSONL/AES-GCM/WAL/Merkle), SafetyGuardrail, decay math, memory processor, snapshot manager, optimized store, and the module registry (lib.rs) plus build script.",
        "nodeIds": []
    },
    "layer:memory-subsystems": {
        "name": "Memory Subsystems",
        "description": "Temporal, symbolic, procedural, reflexion (aureus), perception, integration, world-model, loop engine, FSM backend, embedding, query processor, source trust, semantic compression, LLM clients, RocksDB backend, and backend wiring.",
        "nodeIds": []
    },
    "layer:intelligence-layer": {
        "name": "Intelligence Layer",
        "description": "Metacognitive modules: world_model_enhanced (transitions, uncertainty, causal, entity, constraint), self_model (capability, decision, health, performance, resource), coherence (checker, resolver, invariants).",
        "nodeIds": []
    },
    "layer:server-surfaces": {
        "name": "Server Surfaces",
        "description": "REST (web_server), gRPC (grpc_server), MCP (mcp_server, mcp_bridge), OpenManus bridge, dashboard, and protobuf schema.",
        "nodeIds": []
    },
    "layer:client-binaries-gui": {
        "name": "Client Binaries & GUI",
        "description": "CLI and webserver binary entrypoints, Tauri GUI, WASM plugin host, and example programs.",
        "nodeIds": []
    },
    "layer:sdks": {
        "name": "SDKs",
        "description": "Python SDK (client, config, daemon, install/MCP helper), TypeScript SDK, Continue.dev adapter.",
        "nodeIds": []
    },
    "layer:vscode-extension": {
        "name": "VS Code Extension",
        "description": "VS Code extension TypeScript source, packaged VSIX artifacts, extension package configs.",
        "nodeIds": []
    },
    "layer:infrastructure-cicd": {
        "name": "Infrastructure & CI/CD",
        "description": "Dockerfiles, Docker Compose stacks, GitHub Actions pipelines, Helm charts, HuggingFace Space, deployment scripts.",
        "nodeIds": []
    },
    "layer:documentation-specs": {
        "name": "Documentation & Specs",
        "description": "README, CLAUDE.md, AGENTS.md, CONTRIBUTING, DEVELOPMENT, DEPLOY, LICENSE, all docs/ and openspec/ proposals/designs/tasks/specs, SDK READMEs, session handover, PRPs, business/launch docs.",
        "nodeIds": []
    },
    "layer:tests-fixtures": {
        "name": "Tests & Fixtures",
        "description": "Unit, integration (SIT/UAT), property, E2E Python harness, benchmark/bench files, JSON fixtures, JSON schemas, and test reports.",
        "nodeIds": []
    },
    "layer:configuration": {
        "name": "Configuration",
        "description": "Cargo.toml, Cargo.lock, .env.example, settings files, SDK package/build configs, tsconfig files, CODEOWNERS.",
        "nodeIds": []
    },
}

def assign(nid):
    p = nid.lower()

    # Intelligence Layer
    for pat in ['world_model_enhanced', 'wme::', '/modules/world_model_enhanced',
                'src/modules/world_model_enhanced', 'src/modules/self_model', 'src/modules/coherence',
                'self_model::', 'self_model_mod', 'coherence::', 'coherence_mod']:
        if pat in p:
            return "layer:intelligence-layer"

    # Server Surfaces
    for pat in ['web_server', 'grpc_server', 'mcp_server', 'mcp_bridge',
                'openmanus_bridge', 'dashboard',
                'proto/memory.proto', 'schema::proto', 'schema:proto']:
        if pat in p:
            return "layer:server-surfaces"

    # VS Code Extension (check before infra to catch all vscode-extension paths)
    if 'vscode-extension' in p:
        return "layer:vscode-extension"

    # Infrastructure & CI/CD
    for pat in ['dockerfile', 'docker-compose', 'huggingface-space',
                '.github/workflows', 'deploy/helm', 'deploy/fly',
                'nix/', 'flake.nix', 'scripts/stamp_versions']:
        if pat in p:
            return "layer:infrastructure-cicd"

    # Tests & Fixtures
    for pat in ['tests/integration', 'tests/unit', 'tests/property',
                'tests/e2e_user_harness', 'tests/test_report',
                'integration_suite', 'unit_suite', 'property_suite',
                ':tests/integration', ':tests/unit', ':tests/property',
                ':tests/e2e', 'sdk/python/tests', ':sdk/python/tests',
                'benchmarks/', 'benches/',
                'config:fixtures/', 'fixtures/sample_',
                'schema:schemas/', 'schemas/perception', 'schemas/procedural',
                'schemas/reflexion', 'schemas/symbolic', 'schemas/temporal',
                'schemas/world_model']:
        if pat in p:
            return "layer:tests-fixtures"

    # SDK source files
    for pat in ['sdk/python/hipcortex', 'sdk/typescript/src', 'sdk/continue/index.ts']:
        if pat in p:
            return "layer:sdks"

    # SDK configs -> Configuration
    for pat in ['sdk/python/pyproject.toml', 'sdk/python/setup.py',
                ':sdk/python/pyproject', ':sdk/python/setup',
                'sdk/typescript/package.json', 'sdk/typescript/tsconfig.json',
                'config:sdk/python/']:
        if pat in p:
            return "layer:configuration"

    # SDK READMEs and SKILL docs -> Documentation
    for pat in ['sdk/python/readme', 'sdk/typescript/readme', 'sdk/continue/readme',
                'sdk/python/hipcortex/install/skill.md', ':sdk/python/readme']:
        if pat in p:
            return "layer:documentation-specs"

    # Documentation & Specs (document: type nodes and known doc files)
    if nid.startswith('document:'):
        return "layer:documentation-specs"
    for pat in ['agents.md', 'benchmark.md', 'claude.md', 'contributing.md',
                'deploy.md', 'development.md', 'hipcortex_env_setup',
                'license', 'production_ready_report', 'quick_start_guide',
                'readme.md', 'docs/', 'openspec/', '.claude/prps/']:
        if pat in p:
            return "layer:documentation-specs"

    # Configuration
    for pat in ['cargo.toml', 'cargo.lock', '.env.example', 'codeowners',
                '.claude/settings', 'config:vscode-extension/package.json',
                'config:vscode-extension/tsconfig.json',
                'config:sdk/typescript/package.json',
                'config:sdk/typescript/tsconfig.json']:
        if pat in p:
            return "layer:configuration"
    if nid.startswith('config:') and not any(x in p for x in
            ['fixtures/', 'schemas/', 'docker', '.github', 'deploy/', 'sdk/']):
        return "layer:configuration"

    # Core Memory Engine
    for pat in ['memory_record', 'memory_store', 'optimized_memory_store',
                'src/decay.rs', ':decay', 'file:decay',
                'memory_processor', 'memory_diff',
                'snapshot_manager', 'safety_guardrail',
                'src/lib.rs', ':src/lib.rs', 'file::src/lib.rs',
                'build.rs', ':build.rs', 'file:build.rs']:
        if pat in p:
            return "layer:core-memory-engine"

    # Memory Subsystems
    for pat in ['temporal_indexer', 'symbolic_store', 'procedural_cache',
                'aureus_bridge', 'perception_adapter', 'integration_layer',
                'world_model', 'loop_engine',
                'temporal_fsm_backend', 'embedding_provider',
                'query_processor', 'source_trust', 'semantic_compression',
                'llm_clients', 'rocksdb_backend',
                'backends_petgraph', 'backends_mod']:
        if pat in p:
            return "layer:memory-subsystems"

    # Client Binaries & GUI
    for pat in ['src/bin/', ':src/bin/', 'src/gui.rs', 'src/plugin_host.rs',
                'examples/', 'memory_cli']:
        if pat in p:
            return "layer:client-binaries-gui"

    # Fallback symbol nodes (no filePath, identified by name in id)
    fallback_core = ['memory_diff', 'memory_processor', 'memory_record',
                     'memory_store', 'snapshot_manager', 'source_trust']
    for sc in fallback_core:
        if sc in p:
            return "layer:core-memory-engine"

    fallback_subsys = ['aureus_bridge', 'coherence_mod', 'integration_layer',
                       'perception_adapter', 'procedural_cache', 'self_model_mod',
                       'symbolic_store', 'temporal_indexer', 'world_model',
                       'temporal_fsm_backend', 'semantic_compression']
    for ss in fallback_subsys:
        if ss in p:
            return "layer:memory-subsystems"

    # Final fallback
    return "layer:documentation-specs"


for nid in all_ids:
    layer = assign(nid)
    layers[layer]["nodeIds"].append(nid)

# Verify coverage
total = len(all_ids)
assigned = sum(len(v["nodeIds"]) for v in layers.values())
print(f"Total: {total}, Assigned: {assigned}")
for lid, ldata in layers.items():
    print(f"  {lid}: {len(ldata['nodeIds'])}")

output = [{"id": lid, "name": ldata["name"], "description": ldata["description"],
           "nodeIds": ldata["nodeIds"]} for lid, ldata in layers.items()]

os.makedirs('D:/all_projects/hipcortex/.ua/intermediate', exist_ok=True)
with open('D:/all_projects/hipcortex/.ua/intermediate/layers.json', 'w', encoding='utf-8') as f:
    json.dump(output, f, indent=2)

print("layers.json written.")
