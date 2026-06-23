# 🧠 HipCortex VS Code Extension

Direct integration between VS Code Copilot chat and HipCortex — the **executable causal topological memory substrate** for AI agents.

HipCortex provides a persistent, self-updating world model with topological reasoning (PPR, Markov blankets, multi-hop paths), Ω Loop Engine for simulation/attribution/self-distillation, and proactive harness so the substrate acts as the primary "mind". LLM (Copilot) is used only as a narrow hypothesis generator for high-entropy cases.

This extension surfaces the full power of the new substrate directly in Copilot chat and tools.

## 🚀 Installation

### 1. Install the Extension
```bash
cd vscode-extension
npm install
npm run compile
```

### 2. Install Extension in VS Code
1. Press `F5` to launch Extension Development Host
2. OR package and install:
```bash
npm install -g vsce
vsce package
code --install-extension hipcortex-memory-0.3.0.vsix
```

### 3. Start HipCortex API Server
```bash
cd ../
cargo run --bin webserver --features "web-server,petgraph_backend"
```

## 🎯 Usage

### Chat Commands

Use `@hipcortex` in VS Code chat followed by commands:

#### Add Memory
```
@hipcortex add actor: CopilotAgent action: code_review target: memory_system
@hipcortex record actor: UserAgent action: completed_task target: project_x
```

#### Query Memory
```
@hipcortex query actor: CopilotAgent
@hipcortex search action: code_review
@hipcortex find actor: VSCode_Chat limit: 5
```

#### System Commands
```
@hipcortex health
@hipcortex status
```

### Command Palette
- `HipCortex: Add Memory Record`
- `HipCortex: Query Memory Records`

## ⚙️ Configuration

Open VS Code Settings and search for "HipCortex":

- **API URL**: Default `http://127.0.0.1:3030`
- **API Key**: Optional authentication key
- **Auto Start**: Automatically start HipCortex server

## 🔧 Features

### ✅ Core Integration
- ✅ VS Code chat participant (@hipcortex)
- ✅ Language Model Tools (hipcortex_search, hipcortex_health, hipcortex_predict, hipcortex_coherence)
- ✅ Command palette integration
- ✅ Auto-start server functionality
- ✅ Comprehensive error handling
- ✅ Health check and status monitoring

### 🧠 New Substrate Features (v0.3.0+)
- **Topological Memory**: Hybrid graph with micro-embeddings, Personalized PageRank, Markov blanket localization, multi-hop causal paths (instead of simple vector search).
- **Ω Loop Engine**: Transactional simulation, Bayesian attribution (topology/policy/utility), error-driven sparse updates, self-distillation of traces + strategies.
- **Memory-Centric Harness**: Substrate as primary mind (live_beliefs, topo reasoning, predictions, coherence). LLM only for final language or creative hyp when entropy is high.
- **Automatic Maintenance**: AgentMessage paths auto-feed world model + trigger loop updates.
- **Epistemic Grounding**: Contradiction detection before integrating hypotheses.

### 🎨 Advanced Features
- 🧠 **Topological reasoning** over persistent causal graph
- 🔄 **Predictive simulation** and counterfactuals via world model + loop
- 📉 **Measurable LLM reduction** (80-99%+ in benchmarks with proactive harness + loop)
- 🔐 **Secure communication** with API authentication
- 🚀 **Auto-recovery** - starts server if not running
- 📊 **Rich formatting** - markdown responses in chat
- ⚡ **Real-time feedback** - streaming responses

## 🔄 Complete Integration Flow

```
VS Code Chat Input
    ↓
@hipcortex add actor: Agent action: task target: system
    ↓
Extension parses command
    ↓
HTTP POST to HipCortex API (/memory/add)
    ↓
HipCortex stores in memory backend
    ↓
Success response back to chat
    ↓
Rich markdown display in VS Code
```

## 🧪 Test Scenarios

### Scenario 1: Basic Memory Operations
1. Type: `@hipcortex health`
2. Expected: ✅ Health check passes
3. Type: `@hipcortex add actor: TestAgent action: integration_test target: extension`
4. Expected: ✅ Memory record created with ID
5. Type: `@hipcortex query actor: TestAgent`
6. Expected: 📊 Shows the record just created

### Scenario 2: Auto-Start Server
1. Stop HipCortex server
2. Type: `@hipcortex health`
3. Expected: ⚠️ Server not responding → 🔄 Auto-starting → ✅ Server started

### Scenario 3: Error Handling
1. Type: `@hipcortex add actor: target: missing_action`
2. Expected: ❌ Input validation error with helpful message

## 📈 Performance

- **Cold start**: ~3-5 seconds (first API call)
- **Warm requests**: ~100-300ms
- **Memory footprint**: ~10-20MB (extension)
- **Network**: HTTP/1.1 with Keep-Alive
- **Token savings**: 80-99%+ with proactive harness + Ω loop (see root benchmarks)

## 🛡️ Security

- ✅ Input validation and sanitization
- ✅ Bearer token authentication support
- ✅ Timeout protection (3s for health checks)
- ✅ Full original prompt is stored in memory metadata (no artificial truncation; previously capped at 500 chars)
- ✅ No sensitive data logged in debug mode

## 🎉 Success Metrics

- **API Integration**: ✅ 100% working
- **Chat Commands**: ✅ All 4 command types implemented
- **Error Handling**: ✅ Graceful degradation
- **User Experience**: ✅ Rich, interactive chat responses
- **Reliability**: ✅ Auto-recovery and health monitoring

## 🚀 v0.3.0+ New Capabilities (Powered by Topological Substrate + Ω Loop)

- Use substrate for **topological reasoning** (PPR, causal paths, localized subgraphs) instead of raw history.
- Trigger **Ω Loop** for simulation, Bayesian attribution, and sparse world-model updates.
- Benefit from **automatic maintenance** and live beliefs — the extension now surfaces the full memory-centric architecture.
- Expect **dramatically lower token usage** in long sessions.

See the [main HipCortex README](https://github.com/farmountain/HipCortex/blob/main/README.md) and [implementation plan](https://github.com/farmountain/HipCortex/blob/main/docs/superpowers/plans/2026-06-20-harness-and-omega-loop-engineering.md) for full details on the topological substrate and Ω Loop.
