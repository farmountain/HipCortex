## 1. Port and Version Utilities

- [x] 1.1 Extract port dynamically from `apiUrl` inside `HipCortexAPI` in `vscode-extension/src/extension.ts`
- [x] 1.2 Implement a cross-platform process killer utility `killProcessOnPort(port: number): Promise<void>` in `vscode-extension/src/extension.ts`

## 2. Server Startup & Verification

- [x] 2.1 Update `doAutoStartServer` to check the version returned by the server on `/health` against the current extension's version
- [x] 2.2 Update `doAutoStartServer` to kill any process occupying the port if the health check fails or the version differs
- [x] 2.3 Pass the dynamically extracted port string in the `PORT` environment variable when spawning `globalServerProcess`

## 3. Server Shutdown Cleanup

- [x] 3.1 Update `deactivate()` to kill `globalServerProcess` if it exists to clean up orphaned server processes

## 4. Compilation & Verification

- [x] 4.1 Compile the VS Code extension using `npm run compile` to verify no TypeScript compilation errors
- [x] 4.2 Run extension unit tests using `npm run test` to verify all tests pass
- [x] 4.3 Rebuild the production bundle using `npm run package`
