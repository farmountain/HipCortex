# Plan: Health body-shape dual contract (C)

## Goal
Mac VS Code extension marks server ready when process is healthy; accept plain `"ok"` and JSON `{status,service,version}`; ship correct bins in VSIX 0.5.6.

## Global constraints
- TDD for extension health parse
- Surgical: health path + fetch-bins + rust health tests
- Do not implement missing worldmodel routes in this change
- EXPECTED_SERVER_VERSION stays crate `0.5.0` (binary version); extension package `0.5.6`
- GitNexus impact on healthCheck: HIGH (doAutoStart, activate) — keep identity rules (foreign service still false)

## Tasks
1. Extension: `parseHealthPayload` + wire `healthCheck` / doAutoStart attach + Jest
2. `fetch-bins.js` force refresh + package script + bump package 0.5.6
3. Rust: health JSON assertion tests; optional shared helper if trivial
