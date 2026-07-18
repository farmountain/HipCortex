## ADDED Requirements

### Requirement: Port Conflict Resolution
The extension SHALL resolve port conflicts dynamically when starting the background server to prevent startup failures and version mismatches.

#### Scenario: Port occupied by healthy matching version
- **WHEN** the extension starts and detects a healthy server running on the configured port with the same version as the extension
- **THEN** the extension SHALL reuse the running server and skip spawning a new process

#### Scenario: Port occupied by old version
- **WHEN** the extension starts and detects a running server on the port with a different version than the extension
- **THEN** the extension SHALL terminate the process occupying the port and spawn the matching version of the server

#### Scenario: Port occupied by hung or unknown process
- **WHEN** the extension starts and detects the port is occupied but the health check fails
- **THEN** the extension SHALL terminate the process occupying the port and spawn the new server

### Requirement: Dynamic Port Extraction
The extension SHALL extract the port dynamically from the configured `hipcortex.apiUrl` instead of using a hardcoded port.

#### Scenario: Custom port configured
- **WHEN** the user configures `hipcortex.apiUrl` to a custom port (e.g., `http://127.0.0.1:3040`)
- **THEN** the extension SHALL use that port for health checks, port conflict detection, and server spawning

### Requirement: Graceful Cleanup on Deactivation
The extension SHALL clean up the spawned server process on deactivation to prevent orphaned processes from occupying resources.

#### Scenario: Extension deactivates
- **WHEN** the extension is deactivated (due to reloading the window, extension updates, or closing VS Code)
- **THEN** the extension SHALL terminate the server process if it was spawned during this session
