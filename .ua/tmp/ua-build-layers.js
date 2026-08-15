#!/usr/bin/env node
// Build layer assignments for HipCortex knowledge graph
// Reads knowledge-graph.json, produces layers.json and an updated knowledge-graph.json

const fs = require('fs');
const KG_PATH = 'D:/all_projects/hipcortex/.ua/knowledge-graph.json';
const LAYERS_PATH = 'D:/all_projects/hipcortex/.ua/intermediate/layers.json';

const kg = JSON.parse(fs.readFileSync(KG_PATH, 'utf8'));
const FILE_LEVEL_TYPES = new Set(['service','config','pipeline','document','resource','schema','file']);
const fileNodes = kg.nodes.filter(n => FILE_LEVEL_TYPES.has(n.type));

// ── Layer definitions ────────────────────────────────────────────────────────
// 9 layers covering the HipCortex architecture

const layers = [
  {
    id: 'layer:storage-persistence',
    name: 'Storage & Persistence',
    description: 'Core memory record types, store implementations (JSONL, encrypted, WAL, Merkle audit), backend trait and RocksDB backend, snapshot management, semantic compression, and data-integrity fixtures and schemas.',
    nodeIds: []
  },
  {
    id: 'layer:memory-subsystems',
    name: 'Memory Subsystems',
    description: 'The four primary memory subsystems: TemporalIndexer (ring-buffer decay), SymbolicStore (graph DB), ProceduralCache (FSM traces), and PerceptionAdapter (multimodal input normalisation), plus the LLM connector registry.',
    nodeIds: []
  },
  {
    id: 'layer:intelligence',
    name: 'Intelligence Layer',
    description: 'Metacognitive modules: SelfModel (capability registry, resource monitor, decision engine), WorldModelEnhanced (Dirichlet-Multinomial transitions, Kalman entity tracking, causal do-calculus), and CoherenceChecker (consistency, conflict resolution, system invariants).',
    nodeIds: []
  },
  {
    id: 'layer:cognitive-runtime',
    name: 'Cognitive Runtime',
    description: 'Active reasoning loop (ReactEngine / loop_engine), AureusBridge reflexion hook, cognitive garbage collector, execution gate, memory diff/delta tracking, world-model actor, and cross-cutting safety guardrail.',
    nodeIds: []
  },
  {
    id: 'layer:api-integration',
    name: 'API & Integration',
    description: 'External-facing surfaces: REST/gRPC/MCP servers (Axum + Tonic), MCP and OpenManus protocol bridges, integration layer for API-key and OAuth2 routing, and the crate entry point (lib.rs, build.rs).',
    nodeIds: []
  },
  {
    id: 'layer:cli-examples',
    name: 'CLI & Examples',
    description: 'Command-line binary (memory_cli), web-server binary entry point, and runnable example programs demonstrating quickstart, RAG export, world-model, and MCP server usage.',
    nodeIds: []
  },
  {
    id: 'layer:sdk-extensions',
    name: 'SDK & Extensions',
    description: 'Client SDKs for TypeScript, Python, and Continue.dev, plus the VS Code extension (TypeScript source and packaged VSIX releases) that surfaces HipCortex memory inside an IDE.',
    nodeIds: []
  },
  {
    id: 'layer:tests',
    name: 'Tests',
    description: 'All test code: Rust unit, integration (SIT/UAT), and property (proptest) suites, Python e2e harness, and test-report documents capturing coverage and regression results.',
    nodeIds: []
  },
  {
    id: 'layer:infrastructure-docs',
    name: 'Infrastructure & Documentation',
    description: 'Deployment artifacts (Dockerfile, docker-compose, Helm charts, HuggingFace space), CI/CD pipelines, project configuration (Cargo.toml, env, CODEOWNERS), OpenSpec change proposals, and all documentation (README, docs/, openspec/, .claude/PRPs).',
    nodeIds: []
  }
];

// ── Assignment map: node ID → layer ID ──────────────────────────────────────
// Build by explicit group → layer mapping, then handle remainder by ID pattern

function layerFor(node) {
  const id = node.id;
  const file = node.file || node.filePath || '';
  const type = node.type;

  // ── Infrastructure & Docs ──────────────────────────────────────────────
  if (type === 'pipeline') return 'layer:infrastructure-docs';          // all CI pipelines
  if (type === 'resource') return 'layer:infrastructure-docs';          // helm templates
  if (id === 'service:Dockerfile') return 'layer:infrastructure-docs';
  if (id === 'service:huggingface-space/Dockerfile') return 'layer:infrastructure-docs';
  if (/docker-compose/.test(id)) return 'layer:infrastructure-docs';
  if (/deploy\/helm/.test(id) || /^(config|file):(:?)deploy\//.test(id)) return 'layer:infrastructure-docs';
  if (id === 'config:.env.example') return 'layer:infrastructure-docs';
  if (id === 'config:CODEOWNERS') return 'layer:infrastructure-docs';
  if (id === 'config:Cargo.toml') return 'layer:infrastructure-docs';
  if (id === 'file:Cargo.lock') return 'layer:infrastructure-docs';
  if (id === 'file:build.rs' || id === 'file::build.rs') return 'layer:api-integration'; // build.rs generates proto
  if (id === 'config:.claude/settings.local.json') return 'layer:infrastructure-docs';
  if (id === 'file:scripts/stamp_versions.py') return 'layer:infrastructure-docs';

  // All document nodes → infra & docs (except test reports which go to tests)
  if (type === 'document') {
    if (/tests\/test_report/.test(id)) return 'layer:tests';
    return 'layer:infrastructure-docs';
  }

  // ── Schemas & Proto ───────────────────────────────────────────────────
  if (type === 'schema') return 'layer:storage-persistence'; // JSON schemas for memory types
  if (id === 'schema::proto/memory.proto' || id === 'schema:proto/memory.proto') return 'layer:api-integration';

  // ── Fixtures ──────────────────────────────────────────────────────────
  if (/^config:fixtures\//.test(id)) return 'layer:storage-persistence';

  // ── SDK & Extensions ──────────────────────────────────────────────────
  if (/^(file|config|document):sdk\//.test(id) ||
      /^(file|config|document):file:sdk\//.test(id) ||
      id.startsWith('file:file:sdk/') ||
      id.startsWith('config:config:vscode-extension/') ||
      /vscode-extension/.test(id) ||
      id.startsWith('file::sdk/')) return 'layer:sdk-extensions';

  // ── Tests ─────────────────────────────────────────────────────────────
  if (/^file::tests\//.test(id) ||
      /^file:tests\//.test(id) ||
      /^file:file::tests\//.test(id) ||
      /^file:file:tests\//.test(id)) return 'layer:tests';

  // ── CLI & Examples ────────────────────────────────────────────────────
  if (/^file:examples\//.test(id)) return 'layer:cli-examples';
  if (id === 'file::src/bin/cli.rs' || id === 'file:memory_cli') return 'layer:cli-examples';
  if (id === 'file::src/bin/webserver.rs') return 'layer:cli-examples';

  // ── API & Integration ─────────────────────────────────────────────────
  if (id === 'file::src/lib.rs' || id === 'file:src/lib.rs') return 'layer:api-integration';
  if (id === 'file::src/web_server.rs' || id === 'file:src/web_server.rs') return 'layer:api-integration'; // wait — web_server is actually in integration
  if (id === 'file::src/mcp_server.rs' || id === 'file:src/mcp_server.rs') return 'layer:api-integration';
  if (id === 'file::src/grpc_server.rs' || id === 'file:src/grpc_server.rs') return 'layer:api-integration';
  if (id === 'file:src/modules/integration_layer.rs' || id === 'file:integration_layer') return 'layer:api-integration';
  if (id === 'file:src/modules/mcp_bridge.rs') return 'layer:api-integration';
  if (id === 'file:src/modules/openmanus_bridge.rs') return 'layer:api-integration';

  // ── Storage & Persistence ─────────────────────────────────────────────
  if (id === 'file::src/memory_record.rs' || id === 'file:src/memory_record.rs' || id === 'file:memory_record') return 'layer:storage-persistence';
  if (id === 'file::src/memory_store.rs' || id === 'file:src/memory_store.rs' || id === 'file:memory_store') return 'layer:storage-persistence';
  if (id === 'file:src/optimized_memory_store.rs') return 'layer:storage-persistence';
  if (id === 'file:src/rocksdb_backend.rs') return 'layer:storage-persistence';
  if (id === 'file:src/snapshot_manager.rs') return 'layer:storage-persistence';
  if (id === 'file:src/semantic_compression.rs') return 'layer:storage-persistence';
  if (id === 'file:src/source_trust.rs') return 'layer:storage-persistence';
  if (id === 'file:src/decay.rs') return 'layer:storage-persistence';
  if (id === 'file:src/embedding_provider.rs') return 'layer:storage-persistence';

  // ── Memory Subsystems ─────────────────────────────────────────────────
  if (id === 'file:src/modules/temporal_indexer.rs' || id === 'file:temporal_indexer') return 'layer:memory-subsystems';
  if (id === 'file:src/modules/symbolic_store.rs'   || id === 'file:symbolic_store')   return 'layer:memory-subsystems';
  if (id === 'file:src/modules/procedural_cache.rs' || id === 'file:procedural_cache') return 'layer:memory-subsystems';
  if (id === 'file:src/modules/perception_adapter.rs'||id === 'file:perception_adapter')return 'layer:memory-subsystems';
  if (id === 'file:src/modules/world_model.rs'      || id === 'file:world_model')      return 'layer:memory-subsystems';
  if (id === 'file:src/llm_clients/mod.rs') return 'layer:memory-subsystems';

  // ── Intelligence Layer ────────────────────────────────────────────────
  if (/self_model/.test(id)) return 'layer:intelligence';
  if (/world_model_enhanced/.test(id) || /^file:wme::/.test(id)) return 'layer:intelligence';
  if (/coherence/.test(id)) return 'layer:intelligence';

  // ── Cognitive Runtime ─────────────────────────────────────────────────
  if (id === 'file:src/modules/aureus_bridge.rs' || id === 'file:aureus_bridge') return 'layer:cognitive-runtime';
  if (id === 'file:src/modules/loop_engine.rs') return 'layer:cognitive-runtime';
  if (id === 'file:src/actors/world_model_actor.rs') return 'layer:cognitive-runtime';
  if (id === 'file:src/memory_diff.rs' || id === 'file:memory_diff' || id === 'file::src/memory_diff.rs') return 'layer:cognitive-runtime';
  if (id === 'file:src/memory_processor.rs' || id === 'file:memory_processor') return 'layer:cognitive-runtime';
  if (id === 'file:src/safety_guardrail.rs' || id === 'file::src/safety_guardrail.rs') return 'layer:cognitive-runtime';
  if (id === 'file:src/plugin_host.rs') return 'layer:cognitive-runtime';
  if (id === 'file:src/gui.rs') return 'layer:cognitive-runtime';
  if (id === 'file:src/dashboard.rs') return 'layer:cognitive-runtime';

  // ── Catch-all for remaining src files ────────────────────────────────
  if (/^file:(file:)?src\//.test(id)) return 'layer:cognitive-runtime';

  // Final fallback
  return 'layer:infrastructure-docs';
}

// Assign all file nodes
const assigned = {};
fileNodes.forEach(n => {
  const layerId = layerFor(n);
  if (!assigned[layerId]) assigned[layerId] = [];
  assigned[layerId].push(n.id);
});

// Populate layers
layers.forEach(l => {
  l.nodeIds = assigned[l.id] || [];
});

// Verify total
const total = layers.reduce((s, l) => s + l.nodeIds.length, 0);
console.error('Total assigned:', total, '/ expected:', fileNodes.length);
if (total !== fileNodes.length) {
  // Find unassigned
  const assignedSet = new Set(layers.flatMap(l => l.nodeIds));
  fileNodes.forEach(n => { if (!assignedSet.has(n.id)) console.error('UNASSIGNED:', n.id); });
}

// Write layers.json
fs.writeFileSync(LAYERS_PATH, JSON.stringify(layers, null, 2));
console.error('Wrote', LAYERS_PATH);

// Print summary
layers.forEach(l => console.log(l.name, ':', l.nodeIds.length, 'nodes'));
console.log('Total:', total);
