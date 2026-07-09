## 1. Extension API updates

- [x] 1.1 Add `searchMemory()` method to `HipCortexAPI` in `vscode-extension/src/extension.ts` using `POST /memory/search`
- [x] 1.2 Add `hipcortex_causal` LM tool to `vscode-extension/src/extension.ts` with `mode` mapping

## 2. Dispatch split and display

- [x] 2.1 Update command dispatcher in `HipCortexChatParticipant` to separate `query` and `search` prefixes
- [x] 2.2 Implement `handleSearchMemory()` method to display scored results and tokens saved footer
- [x] 2.3 Verify extension compiles successfully using `npm run compile`
