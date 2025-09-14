# HipCortex Development Guide

## Quick Development Setup

### Prerequisites
- Rust 1.70+ (`cargo --version`)
- Git
- Visual Studio Build Tools (Windows)

### Fast Setup (No External Dependencies)
```bash
# Clone and build with minimal features
git clone https://github.com/farmountain/HipCortex.git
cd HipCortex
cargo build --no-default-features --features "petgraph_backend"

# Verify setup
cargo run --example quickstart --no-default-features --features "petgraph_backend"
cargo test --no-default-features --features "petgraph_backend" --lib
```

## Development Commands

### Building
```bash
# Minimal build (in-memory graph only)
cargo build --no-default-features --features "petgraph_backend"

# With web server
cargo build --no-default-features --features "web-server,petgraph_backend"

# With GUI
cargo build --no-default-features --features "gui,petgraph_backend"

# All features (requires external database libraries)
cargo build --all-features
```

### Testing
```bash
# Fast tests (minimal features)
cargo test --no-default-features --features "petgraph_backend" --lib

# All tests
cargo test --no-default-features --features "petgraph_backend"

# Integration tests
cargo test --no-default-features --features "petgraph_backend" --test integration_suite

# Benchmarks
cargo bench --no-default-features --features "petgraph_backend"
```

### Running Examples
```bash
# Basic functionality
cargo run --example quickstart --no-default-features --features "petgraph_backend"

# World model demonstration
cargo run --example world_model_example --no-default-features --features "petgraph_backend"

# RAG export functionality
cargo run --example rag_export --no-default-features --features "petgraph_backend"

# Plugin system (requires plugin feature)
cargo run --example plugin_host --no-default-features --features "plugin,petgraph_backend"

# Web server (requires external features)
cargo run --example mcp_server --no-default-features --features "web-server,petgraph_backend"
```

## Feature Flags

| Feature | Description | External Dependencies |
|---------|-------------|----------------------|
| `petgraph_backend` | In-memory graph storage | None |
| `sqlite_backend` | SQLite database support | SQLite development libraries |
| `postgres_backend` | PostgreSQL database support | PostgreSQL development libraries |
| `neo4j_backend` | Neo4j graph database support | Neo4j server |
| `web-server` | REST API server | None |
| `gui` | Tauri desktop application | Node.js, Tauri dependencies |
| `grpc-server` | gRPC server | None |
| `plugin` | WebAssembly plugin system | None |
| `parallel` | Parallel processing support | None |
| `gpu` | GPU acceleration | DirectX 12 compatible hardware |

## Database Setup (Optional)

### SQLite
```bash
# Install SQLite development libraries
# Then build with: --features "petgraph_backend,sqlite_backend"
```

### PostgreSQL
```bash
# Install PostgreSQL and development headers
# Then build with: --features "petgraph_backend,postgres_backend"
```

### Neo4j
```bash
# Start Neo4j server (Docker or desktop)
docker run -d --name neo4j-dev -p 7474:7474 -p 7687:7687 -e NEO4J_AUTH=neo4j/testpass neo4j:latest
# Then build with: --features "petgraph_backend,neo4j_backend"
```

## Code Structure

- `src/lib.rs` - Main library exports
- `src/modules/` - Core memory modules
- `src/backends/` - Database backend implementations
- `examples/` - Runnable examples
- `tests/` - Test suites
- `benches/` - Performance benchmarks

## Troubleshooting

### Common Issues
1. **Linker errors**: Install Visual Studio Build Tools
2. **Database errors**: Start with `--no-default-features --features "petgraph_backend"`
3. **Missing dependencies**: Use minimal feature set first

### Getting Help
- Check `Hipcortex_Env_Setup_Guide.md` for detailed setup
- Review examples in `examples/` directory
- Run tests to verify functionality

## Performance Tips

- Use `petgraph_backend` for development and small datasets
- Enable `parallel` feature for CPU-intensive tasks
- Use `rocksdb-backend` or external databases for large datasets
- Enable `gpu` feature if you have compatible hardware
