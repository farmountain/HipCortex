## Why

The HipCortex VS Code extension automatically starts and manages a background local memory server. Currently, when the extension is updated or reloaded, the old server process may remain orphaned and continue to occupy the configured port. This causes port conflicts, version drift, and startup failures in subsequent sessions. Robust process lifecycle, port conflict resolution, and version validation are required to ensure continuous service.

## What Changes

- **Version-Aware Autostart**: Compare the version of any running server (via `/health`) with the active extension version. If they differ, the server must be restarted to avoid API/schema drift.
- **Port Conflict Resolution**: Detect if the port is occupied by an old or unhealthy server process. If so, identify the PID and terminate the process holding the port before attempting to spawn the new server.
- **Dynamic Port Selection**: Read the port dynamically from the extension's configured `apiUrl` instead of using a hardcoded `3030`.
- **Graceful Cleanup on Deactivation**: Terminate the spawned child server process when the VS Code extension is deactivated (e.g. reload window, extension update, or close).

## Capabilities

### New Capabilities
- `server-lifecycle`: Manage background memory server lifecycle, port allocation, version verification, and recovery in a cross-platform manner.

### Modified Capabilities

## Impact

- **Affected Code**: `vscode-extension/src/extension.ts` (API client, server auto-start logic, activation, deactivation).
- **Dependencies**: Uses native Node.js APIs (`child_process`, `os`, `axios`) to discover and terminate conflicting port processes across Windows, macOS, and Linux.
