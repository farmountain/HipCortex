# HipCortex Environment Setup Guide

Based on analysis of the entire HipCortex codebase, here's a comprehensive guide to prepare your environment for testing this AI memory engine project.

## Quick Start (Minimal Setup)

If you want to get started quickly without external database dependencies:

```powershell
# 1. Ensure Rust is installed
cargo --version

# 2. Clone and navigate to project
git clone https://github.com/farmountain/HipCortex.git
cd HipCortex

# 3. Build with minimal features
cargo build --no-default-features --features "petgraph_backend"

# 4. Test the setup
cargo run --example quickstart --no-default-features --features "petgraph_backend"

# 5. Run tests
cargo test --no-default-features --features "petgraph_backend" --lib
```

This uses in-memory graph storage and avoids external database dependencies.

## Full Setup (All Features)

For the complete experience with all database backends and features, follow the detailed setup below.
## 1. Core Prerequisites

### 1.1 Rust Development Environment
```powershell
# Install Rust toolchain (minimum version 1.70)
winget install Rustlang.Rust.MSVC

# Verify installation
cargo --version
rustc --version

# Install additional Rust components
rustup component add clippy rustfmt
```

### 1.2 System Dependencies for Windows
```powershell
# Install Visual Studio Build Tools (required for native dependencies)
winget install Microsoft.VisualStudio.2022.BuildTools

# Install Git (if not already installed)
winget install Git.Git

# Install Node.js and npm (required for Tauri GUI)
winget install OpenJS.NodeJS

# Install Python (required for some build scripts)
winget install Python.Python.3.11
```

## 2. Database Setup Options

HipCortex supports multiple database backends. Choose based on your testing needs:

### 2.1 Basic File-based Storage (Default)
- **No setup required** - uses JSONL files for persistence
- Suitable for: Basic testing, development, small datasets

### 2.2 RocksDB Backend (Embedded)
```powershell
# No external setup required - RocksDB is embedded
# Enable with: cargo build --features rocksdb-backend
```

### 2.3 PostgreSQL Backend (Optional)
```powershell
# Install PostgreSQL
winget install PostgreSQL.PostgreSQL

# Start PostgreSQL service
net start postgresql-x64-14

# Create test database
psql -U postgres -c "CREATE DATABASE hipcortex_test;"

# Set environment variable for testing
$env:POSTGRES_TEST_URL = "postgresql://postgres:password@localhost/hipcortex_test"
```

### 2.4 Neo4j Backend (Optional)
```powershell
# Install Neo4j Desktop or Community Edition
winget install Neo4j.Neo4jDesktop

# Or using Docker:
docker run -d --name neo4j-test -p 7474:7474 -p 7687:7687 -e NEO4J_AUTH=neo4j/testpass neo4j:latest

# Set environment variables for testing
$env:NEO4J_TEST_URI = "bolt://localhost:7687"
$env:NEO4J_TEST_USER = "neo4j"
$env:NEO4J_TEST_PASS = "testpass"
```

## 3. Feature-Specific Dependencies

### 3.1 Web Server Features
```powershell
# Required for REST API testing
# Dependencies handled automatically via Cargo
```

### 3.2 GUI (Tauri) Dependencies
```powershell
# Install Tauri prerequisites
npm install -g @tauri-apps/cli

# Install Yarn (alternative package manager)
npm install -g yarn

# Verify Tauri setup
cargo tauri info
```

### 3.3 gRPC Server Dependencies
```powershell
# Install Protocol Buffers compiler
winget install Google.ProtocolBuffers

# Verify protoc installation
protoc --version
```

### 3.4 GPU Support (Optional)
```powershell
# For GPU-accelerated vision encoding
# Ensure you have DirectX 12 compatible hardware
# Install latest GPU drivers from manufacturer
```

## 4. LLM Integration Setup

### 4.1 OpenAI API
```powershell
# Set your OpenAI API key
$env:OPENAI_API_KEY = "your-api-key-here"

# Add to your PowerShell profile for persistence
Add-Content $PROFILE '$env:OPENAI_API_KEY = "your-api-key-here"'
```

### 4.2 Local LLM Support (Ollama)
```powershell
# Install Ollama for local LLM testing
winget install Ollama.Ollama

# Pull a model for testing
ollama pull llama2:7b
```

## 5. Project Setup

### 5.1 Clone and Build
```powershell
# Clone the repository
git clone https://github.com/farmountain/HipCortex.git
cd HipCortex

# Install dependencies
cargo fetch

<<<<<<< HEAD
# Basic build (minimal features - recommended for initial setup)
cargo build --no-default-features --features "petgraph_backend"

# Verify basic build with all compatible features
=======
# Verify basic build
>>>>>>> origin/main
cargo check --all-features

# Run the startup script equivalent for Windows
cargo check --all-features
```

<<<<<<< HEAD
### 5.2 Quick Start (Minimal Setup)
For fastest setup without external database dependencies:
```powershell
# Build with minimal features (in-memory graph storage)
cargo build --no-default-features --features "petgraph_backend"

# Test the setup
cargo run --example quickstart --no-default-features --features "petgraph_backend"

# Run basic tests
cargo test --no-default-features --features "petgraph_backend" --lib
```

=======
>>>>>>> origin/main
### 5.2 Environment Variables Setup
```powershell
# Create a .env file or set in PowerShell session
$env:RUST_LOG = "debug"
$env:RUST_BACKTRACE = "1"

# For database testing (if using external databases)
$env:DATABASE_URL = "your-database-connection-string"

# For encryption testing
$env:HIPCORTEX_MASTER_KEY = "your-32-byte-hex-key"
```

## 6. Build Configurations

<<<<<<< HEAD
### 6.1 Minimal Build (Recommended for Development)
```powershell
# In-memory graph backend only (no external database dependencies)
cargo build --no-default-features --features "petgraph_backend"
```

### 6.2 Database Backend Builds
```powershell
# SQLite backend (requires SQLite development libraries)
cargo build --no-default-features --features "petgraph_backend,sqlite_backend"

# PostgreSQL backend (requires PostgreSQL development libraries)
cargo build --no-default-features --features "petgraph_backend,postgres_backend"

# Neo4j backend (requires Neo4j server)
cargo build --no-default-features --features "petgraph_backend,neo4j_backend"
```

### 6.3 Full Feature Build
```powershell
# All features (requires all external dependencies)
cargo build --all-features
```

### 6.4 Specific Feature Builds
```powershell
# Web server with REST API
cargo build --no-default-features --features "web-server,petgraph_backend"

# GUI application
cargo build --no-default-features --features "gui,petgraph_backend"

# gRPC server
cargo build --no-default-features --features "grpc-server,petgraph_backend"

# Plugin system
cargo build --no-default-features --features "plugin,petgraph_backend"

# GPU acceleration
cargo build --no-default-features --features "gpu,petgraph_backend"

# Parallel processing
cargo build --no-default-features --features "parallel,petgraph_backend"
=======
### 6.1 Minimal Build (Default)
```powershell
cargo build
```

### 6.2 Full Feature Build
```powershell
cargo build --all-features
```

### 6.3 Specific Feature Builds
```powershell
# Web server with REST API
cargo build --features web-server

# GUI application
cargo build --features gui

# Database backends
cargo build --features "rocksdb-backend,postgres_backend,neo4j_backend"

# gRPC server
cargo build --features grpc-server

# Plugin system
cargo build --features plugin

# GPU acceleration
cargo build --features gpu

# Parallel processing
cargo build --features parallel
>>>>>>> origin/main
```

## 7. Testing Environment Verification

<<<<<<< HEAD
### 7.1 Run Basic Tests (Minimal Setup)
```powershell
# Unit tests with minimal features
cargo test --no-default-features --features "petgraph_backend" --lib

# Integration tests (basic)
cargo test --no-default-features --features "petgraph_backend" --test integration_suite

# All tests with minimal features
cargo test --no-default-features --features "petgraph_backend"
```

### 7.2 Run All Tests (Full Setup)
```powershell
# Unit tests (requires all dependencies)
cargo test

# Integration tests (requires all dependencies)
cargo test --test integration_suite

# Property-based tests (requires all dependencies)
cargo test --test property_suite

# With all features (requires all external dependencies)
cargo test --all-features
```

### 7.3 Run Benchmarks
```powershell
# Basic benchmarks
cargo bench --no-default-features --features "petgraph_backend"

# All benchmarks (requires all dependencies)
cargo bench
```

### 7.4 Run Examples
```powershell
# Basic quickstart (minimal dependencies)
cargo run --example quickstart --no-default-features --features "petgraph_backend"

# World model example (minimal dependencies)
cargo run --example world_model_example --no-default-features --features "petgraph_backend"

# RAG export (minimal dependencies)
cargo run --example rag_export --no-default-features --features "petgraph_backend"

# Plugin host (requires plugin feature)
cargo run --example plugin_host --no-default-features --features "plugin,petgraph_backend"

# MCP server (requires web server features and external databases)
cargo run --example mcp_server --features "web-server,grpc-server,petgraph_backend"
=======
### 7.1 Run All Tests
```powershell
# Unit tests
cargo test

# Integration tests
cargo test --test integration_suite

# Property-based tests
cargo test --test property_suite

# With all features
cargo test --all-features
```

### 7.2 Run Benchmarks
```powershell
cargo bench
```

### 7.3 Run Examples
```powershell
# Basic quickstart
cargo run --example quickstart

# MCP server (REST + gRPC)
cargo run --example mcp_server --features "web-server,grpc-server"

# World model example
cargo run --example world_model_example

# RAG export
cargo run --example rag_export

# Plugin host
cargo run --example plugin_host --features plugin
>>>>>>> origin/main
```

## 8. IDE Setup (VS Code)

### 8.1 Required Extensions
```powershell
# Install VS Code extensions
code --install-extension rust-lang.rust-analyzer
code --install-extension vadimcn.vscode-lldb
code --install-extension tamasfe.even-better-toml
```

### 8.2 Workspace Configuration
The project includes `.vscode/` configuration for:
- Debug configurations
- Task definitions
- Settings optimization

## 9. Performance and Monitoring Setup

### 9.1 Enable Monitoring
```powershell
# Run with monitoring enabled
cargo run --features web-server --example mcp_server

# Access monitoring at: http://localhost:3000/metrics
```

### 9.2 Memory Profiling Tools
```powershell
# Install heaptrack or similar profiling tools if needed
# On Windows, use Application Verifier or PerfView
```

## 10. Common Issues and Solutions

### 10.1 Build Issues
<<<<<<< HEAD
- **SQLite/Database linking errors**: Use minimal features first: `--no-default-features --features "petgraph_backend"`
=======
>>>>>>> origin/main
- **OpenSSL errors**: Install OpenSSL for Windows
- **Linker errors**: Ensure Visual Studio Build Tools are installed
- **Feature conflicts**: Check feature compatibility in Cargo.toml

### 10.2 Database Connection Issues
<<<<<<< HEAD
- **Start with minimal setup**: Use `petgraph_backend` feature to avoid external database dependencies
- **SQLite errors**: Install SQLite development libraries or use `--no-default-features`
=======
>>>>>>> origin/main
- **PostgreSQL**: Verify service is running and credentials are correct
- **Neo4j**: Ensure Docker container is running or desktop app is started
- **RocksDB**: Check file permissions in the target directory

### 10.3 Performance Issues
- **Large datasets**: Consider using RocksDB backend for better performance
- **Memory usage**: Monitor with `--features parallel` for CPU-intensive tasks
- **GPU acceleration**: Verify DirectX 12 support for vision encoding

<<<<<<< HEAD
### 10.4 Feature-Specific Issues
- **Missing diesel errors**: Use `--no-default-features --features "petgraph_backend"` for minimal setup
- **Database schema errors**: Ensure you have the correct backend features enabled
- **Web server not starting**: Verify tokio and axum features are enabled
- **GUI compilation errors**: Ensure Node.js and Tauri dependencies are installed

=======
>>>>>>> origin/main
## 11. Production Deployment Considerations

### 11.1 Security
- Enable AES-GCM encryption for sensitive data
- Set up proper API key management
- Configure audit logging

### 11.2 Scalability
- Use external databases (PostgreSQL/Neo4j) for large-scale deployments
- Enable async features for high-throughput scenarios
- Consider horizontal scaling with multiple instances

This setup guide covers all the requirements found in the codebase. Start with the basic setup and add features as needed for your specific testing scenarios.
<<<<<<< HEAD

---

## 12. Testing Status Summary

### ✅ **Tested and Working**
- **Core Library**: All 171 tests passing
- **In-memory Graph Backend**: `petgraph_backend` fully functional
- **Examples**: `quickstart`, `world_model_example`, `rag_export` working
- **Basic Features**: Web server, parallel processing, async store compatible
- **Build System**: Cross-platform compilation working

### ⚠️ **Not Yet Tested (Requires External Setup)**
- **PostgreSQL Backend**: Requires PostgreSQL installation and configuration
- **Neo4j Backend**: Requires Neo4j server setup
- **SQLite Backend**: Requires SQLite development libraries
- **Ollama Integration**: Requires Ollama installation and model download
- **OpenAI Integration**: Requires API key configuration
- **GUI Features**: Requires Node.js and Tauri setup
- **gRPC Server**: Requires Protocol Buffers compiler

### 🚀 **Next Steps for Full Testing**
To test external integrations, follow these steps:

```powershell
# Test PostgreSQL backend
winget install PostgreSQL.PostgreSQL
cargo test --no-default-features --features "petgraph_backend,postgres_backend"

# Test Neo4j backend  
docker run -d -p 7474:7474 -p 7687:7687 -e NEO4J_AUTH=neo4j/testpass neo4j:latest
cargo test --no-default-features --features "petgraph_backend,neo4j_backend"

# Test Ollama integration
winget install Ollama.Ollama
ollama pull llama2:7b
cargo test --no-default-features --features "petgraph_backend" llm_clients

# Test all features (requires all external dependencies)
cargo test --all-features
```
=======
>>>>>>> origin/main
