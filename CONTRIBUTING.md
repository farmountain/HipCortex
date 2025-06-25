# Contributing to HipCortex

We welcome new issues and pull requests. Follow these steps to get started.

## Development Setup

```sh
cargo build
cargo test
```

Run `cargo test` before submitting patches. Tests can use lightweight stubs placed in `tests/`.

## Coding Style

- Use Rustfmt and clippy.
- Document public functions.
- Keep modules small and focused.

## Pull Requests

1. Fork the repo and create a feature branch.
2. Add tests or update existing ones.
3. Ensure `cargo test` passes.
4. Open a PR describing your changes.

See `docs/contributing.md` for more detail on contribution policies.
