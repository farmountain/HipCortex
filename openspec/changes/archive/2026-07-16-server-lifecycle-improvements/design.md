## Context

The VS Code extension starts and manages a background local memory server (`hipcortex` binary). Currently, when the extension is updated or reloaded, the old server process may remain orphaned and continue to occupy the configured port. This causes conflicts, version drift, and start failures in subsequent sessions. Robust process lifecycle, port conflict resolution, and version validation are required to ensure continuous service.

## Goals / Non-Goals

**Goals:**
- Dynamically extract port configuration from `apiUrl`.
- Implement version verification via `/health` to detect and replace old server instances.
- Kill conflicting processes on the target port in a cross-platform manner.
- Ensure the server process is killed when the VS Code extension is deactivated.

**Non-Goals:**
- Support running multiple versions of the server on the same port concurrently.
- Standardize process killing on systems without standard shell tools (`netstat`, `lsof`, `taskkill`, `kill`).

## Decisions

### 1. Cross-Platform Port Discovery and Termination
We will implement a Node-based execution runner that checks the OS platform and issues specific CLI commands to extract PIDs and terminate processes occupying the target port:
- **Windows**: Use `netstat -ano | findstr :<port>` to parse PIDs, then `taskkill /F /PID <pid>`.
- **Unix (macOS/Linux)**: Use `lsof -t -i:<port>` to parse PIDs, then `kill -9 <pid>`, falling back to `fuser -k <port>/tcp`.

*Alternatives Considered:*
- Using third-party Node modules like `portfinder` or `kill-port`. *Rejected*: VS Code extensions should minimize heavy external dependencies, and bundling native compiled dependencies is error-prone.
- Changing ports automatically. *Rejected*: This would break client connectivity configurations since other clients expect the configured port.

### 2. Version Verification during Autostart
During activation, the extension will perform a GET request to `${this.baseUrl}/health` and compare the returned `"version"` string with the active extension's version (retrieved via `context.extension.packageJSON.version`). If they match, the server is reused. If they differ, the port is cleared and the new version is spawned.

*Alternatives Considered:*
- Always spawning a new process and letting it fail if the port is busy. *Rejected*: This fails to update the server when the extension is updated.

## Risks / Trade-offs

- **Non-HipCortex Process Termination**:
  - *Risk*: Terminating a non-HipCortex process occupying the port.
  - *Mitigation*: This is acceptable because the configured port is dedicated to HipCortex. If another process is using it, the user can reconfigure `hipcortex.apiUrl` to a different port, and our dynamic port selection will honor it.
- **Restricted Environments**:
  - *Risk*: Missing commands on restricted environments (e.g., no `lsof` or restricted execution policy on Windows).
  - *Mitigation*: Catch any shell execution errors gracefully and log them to the Output channel, falling back to standard spawn with port busy error messages.
