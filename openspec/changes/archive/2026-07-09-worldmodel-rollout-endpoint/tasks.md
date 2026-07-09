## 1. Backend route implementation

- [x] 1.1 Define `WmRolloutRequest` struct in `src/web_server.rs`
- [x] 1.2 Implement `handle_wm_rollout` handler function in `src/web_server.rs`
- [x] 1.3 Register `/worldmodel/rollout` POST route in `build_router()`
- [x] 1.4 Add `/worldmodel/rollout` to the auth whitelist in `src/web_server.rs`

## 2. Extension integration

- [x] 2.1 Register `hipcortex_rollout` LM tool in `vscode-extension/src/extension.ts`
- [x] 2.2 Verify compile and run integration tests
