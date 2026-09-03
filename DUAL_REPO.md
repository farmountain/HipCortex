# Dual-repository architecture

HipCortex keeps a public developer surface and a private engine.

```
[Private]  hipcortex-core   engine, graph index, RK4/twin, orchestration
[Public]   HipCortex        docs, SDKs, connectors, issues, release artifacts
```

| Repo | Visibility | What belongs there |
|------|------------|--------------------|
| [farmountain/HipCortex](https://github.com/farmountain/HipCortex) | Public | README, docs (black-box), OpenAPI, Python/TS SDKs, editor clients, examples, issue tracker, GitHub Releases |
| [farmountain/hipcortex-core](https://github.com/farmountain/hipcortex-core) | Private | `src/`, `Cargo.toml`, engine tests/benches, internal design notes, Dockerfiles that *build* the engine |

## Why this split

GitHub is the discovery and trust surface. The engine is the product. Publishing client libraries and docs does not require publishing retrieval algorithms, graph indexing, or orchestration.

This repository already contains historical engine source under Apache-2.0. That snapshot grant stays in git history. **New** engine work does not land here.

## What stays public

- Install path: `pip install hipcortex` / `npm install hipcortex` / VSIX / release binaries
- HTTP + MCP contracts, schemas, cookbooks
- Black-box architecture (state lifecycle, memory tiers, how to call the substrate)
- Benchmark *results* and a public harness description
- Connectors (LangChain, MCP wrappers, Chrome extension) that cannot run without the runtime

## What does not stay public going forward

- Engine implementation (`src/`, internal crates)
- Internal design notes that specify algorithms (Kalman, Dirichlet internals, module wiring)
- Dockerfiles that compile the engine from this tree
- Working notes, temp data, and completion reports at the repo root

Public `docker-compose` files should *pull* a published image (`ghcr.io/...`), not rebuild `src/`.

## License

| Code | License |
|------|---------|
| This public repo (SDKs, docs, connectors) | Apache-2.0 |
| Engine source already in this repo's git history | Apache-2.0 for those commits |
| New work in `hipcortex-core` | Proprietary unless a file says otherwise. BSL 1.1 is the planned source-available option for on-prem eval. |

Sole copyright holder: Liew Keong Han. Relicensing *future* engine work does not require third-party consent. It does not un-license clones or forks of older Apache-2.0 snapshots.

## Seed the private repo from this tree

Run this on your machine (force-push is required once because `hipcortex-core` was created with a starter commit):

```bash
git clone --bare https://github.com/farmountain/HipCortex.git
cd HipCortex.git
git push --force https://github.com/farmountain/hipcortex-core.git main:main
# optional: copy tags
git push --force https://github.com/farmountain/hipcortex-core.git --tags
```

After the mirror, restore `hipcortex-core`'s own README if the push replaced it.

Do **not** rewrite history on this public repo to hide `src/`. History rewrite will not recall clones and will break existing checkouts.

## After the seed

1. Point engine CI at `hipcortex-core`.
2. Publish binaries and GHCR images from that CI onto *this* repo's Releases.
3. Stop merging engine changes into `farmountain/HipCortex`.
4. Issues and (when enabled) Discussions stay on this public repo.
5. PRs here: docs, SDKs, connectors, schemas — not engine internals.
