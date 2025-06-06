# Codex Agent Guide

This repository is configured for use with the OpenAI Codex agent. The agent should follow the guidelines below when creating pull requests or modifying code.

---

## Workflow

1. Work directly on the `work` branch. Do not create new branches.
2. After making changes, run `cargo test` to ensure all tests pass.
3. Commit your code with a clear message summarizing the change.
4. Create a pull request that includes **Summary** and **Testing** sections describing the update and the result of `cargo test`.

## Formatting & Linting

- Run `cargo fmt` before committing Rust source changes.
- Keep documentation lines under 120 characters when possible.

## Repository Layout

- `src/` – core library modules
- `tests/` – integration and unit tests
- `benches/` – Criterion benchmarks
- `docs/` – project documentation

## Additional Tips

- Prefer small, incremental commits over large monolithic ones.
- Use descriptive variable and function names.
- When adding new modules, include accompanying tests in `tests/`.

Following these steps will help the Codex agent produce consistent contributions to HipCortex.
