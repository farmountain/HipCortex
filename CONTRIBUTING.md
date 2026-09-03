# Contributing to HipCortex

This repository is the **public developer surface**: documentation, client SDKs, connectors, schemas, and examples.

The engine is developed in the private repo `hipcortex-core`. Pull requests that change retrieval algorithms, graph indexing, or backend runtime should not target this repo.

## Welcome here

- Docs, cookbooks, OpenAPI / schema fixes
- Python and TypeScript SDK improvements
- Host adapters (LangChain, MCP wrappers, editor clients)
- Bug reports and reproduction notes via [Issues](https://github.com/farmountain/HipCortex/issues)

## Development setup (public surface)

SDK and docs changes do not require a full engine build. If you are working against a running server:

```sh
pip install -U hipcortex
hipcortex start
hipcortex doctor
```

Historical engine snapshots in this tree still build with:

```sh
cargo build
cargo test
```

Run tests that apply to your change before opening a PR.

## Coding style

- Rust: rustfmt and clippy on any historical engine files you touch.
- Python / TypeScript: match the existing SDK style.
- Document public functions and API contract changes.

## Pull requests

1. Fork the repo and create a feature branch.
2. Keep the change on the public surface unless you are fixing a published snapshot bug.
3. Add or update tests where they exist.
4. Open a PR that says what user-facing behavior changed.

See [DUAL_REPO.md](DUAL_REPO.md) for the split and [docs/contributing.md](docs/contributing.md) if present.
