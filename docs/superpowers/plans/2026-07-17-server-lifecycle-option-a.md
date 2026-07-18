# Server Lifecycle (Option A) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship robust VS Code extension local-server lifecycle — dynamic port from `apiUrl`, safe port reclaim, version-aware reuse/restart, and `deactivate()` child cleanup — without breaking auto-capture or killing healthy mismatched-skew servers incorrectly.

**Architecture:** Pure helpers (`extractPortFromBaseUrl`, `isServerVersionAcceptable`, `shouldReuseRunningServer`) + module-level `killProcessOnPort` + rewrite of `HipCortexAPI.doAutoStartServer` and `deactivate()`. Sits on GitNexus paths **OnSave / Activate / RestartServer → autoStartServer → doAutoStartServer → ensureServerBinary**. Client-only TypeScript in `vscode-extension/`; no Rust unless a follow-up explicitly syncs crate version constants.

**Tech Stack:** TypeScript, VS Code Extension API, Node `child_process` / `axios`, Jest (`vscode-extension/src/test/extension.test.ts`), webpack / `@vscode/vsce`.

## Global Constraints

- **Scope:** Option A lifecycle only. Do **not** commit openspec/sdk/rust/tooling dirt in the same commits.
- **Surgical staging:** working tree may contain other WIP on `extension.ts` — commit **only** lifecycle hunks (or reset to HEAD then apply this plan cleanly).
- **GitNexus blast:** `doAutoStartServer` is on auto-capture (`onSave`) and activate. Regressions break tags write path.
- **Version policy (LOCKED — do not use extension package.json as kill key):**
  - `/health` returns `"version": env!("CARGO_PKG_VERSION")` (crate version, e.g. `0.5.0`).
  - Extension UI version is `package.json` (currently `0.5.3`) — **different product version**.
  - Expected server version = exported constant `EXPECTED_SERVER_VERSION` (must match bundled/release **crate** version used in `server/` binaries).
  - **Default (`strictServerVersion` false):** if `/health` returns `status: 'ok'`, **reuse** unless `health.version` is present **and** fails `isServerVersionAcceptable` (major mismatch or below min).
  - **Missing `health.version`:** treat as **acceptable** when healthy (do not kill).
  - **Strict mode** (`hipcortex.strictServerVersion: true`): require exact string equality `health.version === EXPECTED_SERVER_VERSION` when healthy; else kill+respawn.
- **Kill safety:** only call `killProcessOnPort` when **not** reusing (unhealthy **or** version unacceptable under policy). Never kill when reusing.
- **Port:** parse from `this.baseUrl` (config `hipcortex.apiUrl`); spawn env `PORT: portStr` — never hardcode `'3030'` for spawn when baseUrl has another port.
- **Tests:** `cd vscode-extension; npx jest --forceExit --testPathPatterns=extension.test` — all existing tests green + new lifecycle pure tests.
- **Version bump:** after feature: extension **0.5.4** + VSIX (lifecycle release).
- **Out of scope:** Marketplace publish; changing Rust `/health` shape (already returns crate version); non-extension buckets from brainstorm.

## Spec alignment

Source: `openspec/specs/server-lifecycle/spec.md` + archive proposal.

| Spec scenario | Plan coverage |
|---------------|---------------|
| Healthy matching version → reuse | Task 2 + pure `shouldReuseRunningServer` |
| Different version → kill + spawn | Task 2 with **crate** version constant (not package.json) |
| Health fail / hung → kill + spawn | Task 2 |
| Custom apiUrl port | `extractPortFromBaseUrl` + `PORT` env |
| Deactivate kills spawned child | Task 3 |

## File Map

| File | Action | What |
|------|--------|------|
| `vscode-extension/src/extension.ts` | Modify | Helpers, `killProcessOnPort`, `doAutoStartServer`, `deactivate`, `EXPECTED_SERVER_VERSION` |
| `vscode-extension/src/test/extension.test.ts` | Modify | Pure unit tests for port/version/reuse + killProcess export if testable |
| `vscode-extension/package.json` | Modify | Optional `strictServerVersion` config contribution; version → `0.5.4` in final task |
| `vscode-extension/hipcortex-memory-0.5.4.vsix` | Create | Final package |
| `openspec/specs/server-lifecycle/spec.md` | Modify (optional small) | Purpose line; note crate vs extension version |

**Do not touch:** tags formatters, `extractSemanticTags`, queryMemory (already shipped).

---

## Task 1: Pure port + version policy helpers (TDD)

**Files:**
- Modify: `vscode-extension/src/extension.ts` (module-level, near other exports)
- Test: `vscode-extension/src/test/extension.test.ts`

**Interfaces:**
- Produces:
  - `export const EXPECTED_SERVER_VERSION: string` — set to crate version of bundled binary. **Initial value:** read current `Cargo.toml` `[package].version` at plan time and hardcode same string (e.g. `"0.5.0"` if that is crate version). Comment: `// Keep in sync with Cargo.toml package.version and release hipcortex-* assets`.
  - `export function extractPortFromBaseUrl(baseUrl: string): { port: number; portStr: string }`
  - `export function isServerVersionAcceptable(serverVersion: string | undefined, expected: string, strict: boolean): boolean`
  - `export function shouldReuseRunningServer(opts: { healthy: boolean; serverVersion?: string; expectedVersion: string; strict: boolean }): boolean`

---

- [ ] **Step 1: Write failing tests**

Add import of the new exports next to existing `../extension` imports. Append:

```typescript
describe('server lifecycle helpers', () => {
    test('extractPortFromBaseUrl default 3030', () => {
        expect(extractPortFromBaseUrl('http://127.0.0.1:3030')).toEqual({ port: 3030, portStr: '3030' });
    });

    test('extractPortFromBaseUrl custom port', () => {
        expect(extractPortFromBaseUrl('http://localhost:3040')).toEqual({ port: 3040, portStr: '3040' });
    });

    test('extractPortFromBaseUrl missing port falls back 3030', () => {
        expect(extractPortFromBaseUrl('http://localhost')).toEqual({ port: 3030, portStr: '3030' });
    });

    test('isServerVersionAcceptable: missing version is ok when not strict', () => {
        expect(isServerVersionAcceptable(undefined, EXPECTED_SERVER_VERSION, false)).toBe(true);
        expect(isServerVersionAcceptable('', EXPECTED_SERVER_VERSION, false)).toBe(true);
    });

    test('isServerVersionAcceptable: missing version fails strict', () => {
        expect(isServerVersionAcceptable(undefined, EXPECTED_SERVER_VERSION, true)).toBe(false);
    });

    test('isServerVersionAcceptable: exact match', () => {
        expect(isServerVersionAcceptable(EXPECTED_SERVER_VERSION, EXPECTED_SERVER_VERSION, false)).toBe(true);
        expect(isServerVersionAcceptable(EXPECTED_SERVER_VERSION, EXPECTED_SERVER_VERSION, true)).toBe(true);
    });

    test('isServerVersionAcceptable: major mismatch fails', () => {
        expect(isServerVersionAcceptable('1.0.0', '0.5.0', false)).toBe(false);
    });

    test('isServerVersionAcceptable: same major.minor different patch ok when not strict', () => {
        // e.g. server 0.5.0 vs expected 0.5.1 — acceptable in default mode
        expect(isServerVersionAcceptable('0.5.0', '0.5.1', false)).toBe(true);
        expect(isServerVersionAcceptable('0.5.0', '0.5.1', true)).toBe(false);
    });

    test('shouldReuseRunningServer: healthy + acceptable version', () => {
        expect(shouldReuseRunningServer({
            healthy: true,
            serverVersion: EXPECTED_SERVER_VERSION,
            expectedVersion: EXPECTED_SERVER_VERSION,
            strict: false,
        })).toBe(true);
    });

    test('shouldReuseRunningServer: unhealthy never reuses', () => {
        expect(shouldReuseRunningServer({
            healthy: false,
            serverVersion: EXPECTED_SERVER_VERSION,
            expectedVersion: EXPECTED_SERVER_VERSION,
            strict: false,
        })).toBe(false);
    });

    test('shouldReuseRunningServer: healthy missing version reuses non-strict', () => {
        expect(shouldReuseRunningServer({
            healthy: true,
            serverVersion: undefined,
            expectedVersion: EXPECTED_SERVER_VERSION,
            strict: false,
        })).toBe(true);
    });
});
```

- [ ] **Step 2: Run tests — expect fail**

```powershell
cd vscode-extension
npx jest --forceExit --testPathPatterns=extension.test
```

Expected: import/export failure for new symbols.

- [ ] **Step 3: Implement helpers**

In `extension.ts` (module level, before `HipCortexAPI` class). Set `EXPECTED_SERVER_VERSION` to the **actual** `Cargo.toml` version string in the repo at implementation time:

```typescript
/**
 * Crate / bundled server binary version (CARGO_PKG_VERSION), NOT vscode package.json.
 * Keep in sync with Cargo.toml [package].version and published hipcortex-* assets.
 */
export const EXPECTED_SERVER_VERSION = '0.5.0'; // UPDATE if Cargo.toml differs at impl time

/** Parse listen port from hipcortex.apiUrl / HipCortexAPI.baseUrl. */
export function extractPortFromBaseUrl(baseUrl: string): { port: number; portStr: string } {
    const raw = (baseUrl || '').split(':').pop()?.replace(/[^0-9]/g, '') || '';
    const portStr = raw.length > 0 ? raw : '3030';
    const port = parseInt(portStr, 10);
    if (!Number.isFinite(port) || port <= 0 || port > 65535) {
        return { port: 3030, portStr: '3030' };
    }
    return { port, portStr };
}

function parseSemverMajorMinor(v: string): { major: number; minor: number } | null {
    const m = /^(\d+)\.(\d+)/.exec((v || '').trim());
    if (!m) { return null; }
    return { major: parseInt(m[1], 10), minor: parseInt(m[2], 10) };
}

/**
 * Version accept policy for /health.version vs EXPECTED_SERVER_VERSION.
 * - strict: exact string match required; missing → false
 * - non-strict: missing → true; same major.minor → true; different major → false
 */
export function isServerVersionAcceptable(
    serverVersion: string | undefined,
    expected: string,
    strict: boolean
): boolean {
    const sv = (serverVersion || '').trim();
    if (!sv) {
        return !strict;
    }
    if (strict) {
        return sv === expected;
    }
    if (sv === expected) {
        return true;
    }
    const a = parseSemverMajorMinor(sv);
    const b = parseSemverMajorMinor(expected);
    if (!a || !b) {
        return false;
    }
    return a.major === b.major && a.minor === b.minor;
}

export function shouldReuseRunningServer(opts: {
    healthy: boolean;
    serverVersion?: string;
    expectedVersion: string;
    strict: boolean;
}): boolean {
    if (!opts.healthy) {
        return false;
    }
    return isServerVersionAcceptable(opts.serverVersion, opts.expectedVersion, opts.strict);
}
```

- [ ] **Step 4: Run tests — expect pass**

```powershell
cd vscode-extension
npx jest --forceExit --testPathPatterns=extension.test
```

- [ ] **Step 5: Commit**

```powershell
git add vscode-extension/src/extension.ts vscode-extension/src/test/extension.test.ts
git commit -m "feat(extension): server lifecycle pure port and version policy helpers"
```

---

## Task 2: `killProcessOnPort` + rewrite `doAutoStartServer`

**Files:**
- Modify: `vscode-extension/src/extension.ts` — `killProcessOnPort`, `doAutoStartServer` (HEAD ~L149–240)
- Optional config: `package.json` contributes `hipcortex.strictServerVersion` boolean default `false`

**Interfaces:**
- Consumes: helpers from Task 1
- Mutates: `globalServerProcess` spawn path only when not reusing

---

- [ ] **Step 1: Add `killProcessOnPort` (module level)**

Use the working-tree implementation (win32 `netstat`+`taskkill`, else `lsof`/`fuser`/`kill -9`). Keep as **non-exported** function unless a test needs it — pure helpers already cover policy.

```typescript
async function killProcessOnPort(port: number): Promise<void> {
    const platform = os.platform();
    if (platform === 'win32') {
        return new Promise<void>((resolve) => {
            cp.exec(`netstat -ano | findstr :${port}`, (err, stdout) => {
                if (err || !stdout) {
                    resolve();
                    return;
                }
                const pids = new Set<string>();
                for (const line of stdout.split('\n')) {
                    const parts = line.trim().split(/\s+/);
                    if (parts.length >= 5 && parts[1].endsWith(`:${port}`) && parts[3] === 'LISTENING') {
                        const pid = parts[4];
                        if (pid && /^\d+$/.test(pid) && pid !== '0') {
                            pids.add(pid);
                        }
                    }
                }
                if (pids.size === 0) {
                    resolve();
                    return;
                }
                Promise.all(
                    Array.from(pids).map(
                        pid =>
                            new Promise<void>(res => {
                                cp.exec(`taskkill /F /PID ${pid}`, () => res());
                            })
                    )
                ).then(() => resolve());
            });
        });
    }
    return new Promise<void>((resolve) => {
        cp.exec(`lsof -t -i:${port}`, (err, stdout) => {
            if (err || !stdout) {
                cp.exec(`fuser -k ${port}/tcp`, () => resolve());
                return;
            }
            const pids = stdout
                .trim()
                .split('\n')
                .map(p => p.trim())
                .filter(p => /^\d+$/.test(p));
            if (pids.length === 0) {
                resolve();
                return;
            }
            Promise.all(
                pids.map(
                    pid =>
                        new Promise<void>(res => {
                            cp.exec(`kill -9 ${pid}`, () => res());
                        })
                )
            ).then(() => resolve());
        });
    });
}
```

- [ ] **Step 2: Optional package.json contribution**

Under `contributes.configuration.properties` (same block as other hipcortex settings), add:

```json
"hipcortex.strictServerVersion": {
    "type": "boolean",
    "default": false,
    "description": "When true, only reuse a local server if /health.version exactly matches the expected crate/server binary version. When false (default), reuse any healthy server with same major.minor (or missing version field)."
}
```

- [ ] **Step 3: Replace `doAutoStartServer` body**

Replace the method so logic is:

```typescript
private async doAutoStartServer(outputChannel?: vscode.OutputChannel): Promise<boolean> {
    const config = vscode.workspace.getConfiguration('hipcortex');
    const autoStart = config.get('autoStart', true);
    if (!autoStart) {
        return false;
    }

    const log = (msg: string) => {
        if (outputChannel) {
            outputChannel.appendLine(msg);
        } else {
            console.log(msg);
        }
    };

    const { port, portStr } = extractPortFromBaseUrl(this.baseUrl);
    const strict = config.get<boolean>('strictServerVersion', false);

    let healthy = false;
    let serverVersion: string | undefined;
    try {
        const healthRes = await axios.get(`${this.baseUrl}/health`, { timeout: 2000 });
        if (healthRes.status === 200 && healthRes.data && healthRes.data.status === 'ok') {
            healthy = true;
            if (typeof healthRes.data.version === 'string') {
                serverVersion = healthRes.data.version;
            }
        }
    } catch {
        healthy = false;
    }

    if (
        shouldReuseRunningServer({
            healthy,
            serverVersion,
            expectedVersion: EXPECTED_SERVER_VERSION,
            strict,
        })
    ) {
        log(
            `Reusing active HipCortex server on port ${portStr}` +
                (serverVersion ? ` (version ${serverVersion})` : ' (version unknown)')
        );
        return true;
    }

    if (healthy) {
        log(
            `Server on port ${portStr} not acceptable for policy ` +
                `(version=${serverVersion ?? 'n/a'}, expected crate ${EXPECTED_SERVER_VERSION}, strict=${strict}). Restarting...`
        );
    } else {
        log(`No healthy server on ${this.baseUrl}. Clearing port ${portStr} if occupied, then starting...`);
    }

    try {
        await killProcessOnPort(port);

        log('Starting HipCortex server...');
        const binaryPath = await this.ensureServerBinary(outputChannel);

        const serverDir = path.join(os.homedir(), '.hipcortex-vscode');
        if (!fs.existsSync(serverDir)) {
            fs.mkdirSync(serverDir, { recursive: true });
        }

        if (globalServerProcess) {
            globalServerProcess.kill();
            globalServerProcess = null;
        }

        globalServerProcess = cp.spawn(binaryPath, [], {
            cwd: serverDir,
            env: {
                ...process.env,
                PORT: portStr,
                DATA_DIR: serverDir,
                RUST_LOG: 'info',
            },
            stdio: ['ignore', 'pipe', 'pipe'],
            detached: false,
            windowsHide: true,
        });

        globalServerProcess.on('error', (err) => {
            log(`Server process error: ${err.message}`);
        });
        globalServerProcess.on('exit', (code) => {
            if (code !== null && code !== 0) {
                log(`Server exited with code ${code}`);
            }
            globalServerProcess = null;
        });
        globalServerProcess.stdout?.on('data', (data) => {
            log(`[stdout] ${data.toString().trim()}`);
        });
        globalServerProcess.stderr?.on('data', (data) => {
            log(`[stderr] ${data.toString().trim()}`);
        });

        for (let i = 0; i < SERVER_START_TIMEOUT_SEC; i++) {
            await new Promise(resolve => setTimeout(resolve, 1000));
            if (await this.healthCheck()) {
                log(`Server ready on ${this.baseUrl}`);
                return true;
            }
        }

        const timeoutMsg = `Server did not respond on ${this.baseUrl} within ${SERVER_START_TIMEOUT_SEC}s. Check Output → HipCortex Server.`;
        log(timeoutMsg);
        vscode.window
            .showWarningMessage(
                'HipCortex server failed to start. Open Output → HipCortex Server for details.',
                'Open Output'
            )
            .then(choice => {
                if (choice === 'Open Output' && outputChannel) {
                    outputChannel.show();
                }
            });
        return false;
    } catch (err) {
        const errMsg = `Failed to auto-start HipCortex server: ${err instanceof Error ? err.message : String(err)}`;
        log(errMsg);
        vscode.window.showErrorMessage(errMsg);
        return false;
    }
}
```

**Critical:** do **not** compare `packageJSON.version` to `health.version`.

- [ ] **Step 4: Run full extension unit tests**

```powershell
cd vscode-extension
npx jest --forceExit --testPathPatterns=extension.test
```

Expected: all prior + Task 1 lifecycle tests PASS. Existing `onSave should ensure server...` still green.

- [ ] **Step 5: Commit**

```powershell
git add vscode-extension/src/extension.ts vscode-extension/package.json
git commit -m "feat(extension): dynamic port, version-aware reuse, and port reclaim on autostart"
```

---

## Task 3: `deactivate()` kills spawned child

**Files:**
- Modify: `vscode-extension/src/extension.ts` — `export function deactivate`

**Interfaces:**
- Consumes: module `globalServerProcess`

---

- [ ] **Step 1: Patch deactivate**

Replace:

```typescript
export function deactivate() {
    console.log('🧠 HipCortex Memory Extension deactivated');
}
```

With:

```typescript
export function deactivate() {
    console.log('🧠 HipCortex Memory Extension deactivated');
    if (globalServerProcess) {
        try {
            globalServerProcess.kill();
        } catch {
            // ignore — process may already be dead
        }
        globalServerProcess = null;
    }
}
```

Note: only kills **child spawned by this extension session**, not foreign PIDs (port kill remains autostart-only). Matches spec “if it was spawned during this session.”

- [ ] **Step 2: Run tests**

```powershell
cd vscode-extension
npx jest --forceExit --testPathPatterns=extension.test
```

- [ ] **Step 3: Commit**

```powershell
git add vscode-extension/src/extension.ts
git commit -m "feat(extension): kill spawned server process on deactivate"
```

---

## Task 4: Align EXPECTED_SERVER_VERSION, package 0.5.4, smoke

**Files:**
- Modify: `vscode-extension/package.json` → `"version": "0.5.4"`
- Modify: `EXPECTED_SERVER_VERSION` if Cargo.toml differs (verify with `Select-String -Path Cargo.toml -Pattern '^version'`)
- Create: `vscode-extension/hipcortex-memory-0.5.4.vsix`
- Optional: one-line Purpose fix in `openspec/specs/server-lifecycle/spec.md`

---

- [ ] **Step 1: Verify crate version**

```powershell
Select-String -Path Cargo.toml -Pattern '^version\s*='
```

Set `EXPECTED_SERVER_VERSION` to that value if not already. Document in commit message if adjusted.

- [ ] **Step 2: Bump extension to 0.5.4**

```json
"version": "0.5.4",
```

- [ ] **Step 3: Package + bundle proof**

```powershell
cd vscode-extension
npx jest --forceExit --testPathPatterns=extension.test
npx @vscode/vsce package --no-dependencies --out hipcortex-memory-0.5.4.vsix
```

Proof strings must appear in `extension/dist/extension.js` inside the VSIX:

```powershell
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = [System.IO.Compression.ZipFile]::OpenRead("$pwd\hipcortex-memory-0.5.4.vsix")
$entry = $zip.GetEntry("extension/dist/extension.js")
$r = [System.IO.StreamReader]::new($entry.Open())
$js = $r.ReadToEnd(); $r.Close(); $zip.Dispose()
@('EXPECTED_SERVER_VERSION','extractPortFromBaseUrl','Reusing active HipCortex','strictServerVersion','taskkill') | ForEach-Object {
  if ($js.Contains($_)) { "PASS: $_" } else { "FAIL: $_" }
}
```

Note: minifiers may rename locals; prefer unique log strings and config key `strictServerVersion` / `Reusing active HipCortex`. Adjust proof list if webpack mangles — require at least `Reusing active HipCortex` and `strictServerVersion`.

- [ ] **Step 4: Commit**

```powershell
git add vscode-extension/package.json vscode-extension/hipcortex-memory-0.5.4.vsix vscode-extension/src/extension.ts
git commit -m "chore: bump extension to v0.5.4 — server lifecycle"
```

- [ ] **Step 5: Manual smoke (implementer documents; human or agent with server)**

1. Install VSIX 0.5.4, reload.
2. With healthy server on 3030 (any patch same major.minor crate version): Output should log **Reusing** — not kill loop.
3. Set `apiUrl` to `http://127.0.0.1:3040` (free port): activate/save should spawn with `PORT=3040`.
4. Reload window: no orphan spam; deactivate should clear child if this session spawned it.
5. Auto-capture still works: save file → tags still written (regression).

- [ ] **Step 6: Push (only after smoke)**

```powershell
git status -sb
git log origin/main..HEAD --oneline
git push origin main
```

Push **only** lifecycle commits; leave other dirt unstaged.

---

## Self-Review

**Spec coverage:**
- ✅ Reuse healthy matching / acceptable version
- ✅ Kill+spawn on unacceptable version (strict or major mismatch)
- ✅ Kill+spawn on health failure
- ✅ Dynamic port from apiUrl
- ✅ Deactivate kills **session** child only
- ✅ Version policy avoids package.json vs CARGO_PKG_VERSION trap

**Non-goals:**
- ❌ Exact equality extension 0.5.4 === health 0.5.0
- ❌ SDK / Rust / openspec junk commits
- ❌ Tags feature rework

**Type consistency:**
- Helpers pure; `doAutoStartServer` only orchestration
- `EXPECTED_SERVER_VERSION` string used in reuse decision only

**Risks called out for implementer:**
- WT may already contain incomplete lifecycle (compared package.json to health) — **replace** with this policy, do not ship WT as-is.
- `killProcessOnPort` is destructive on shared ports — default reuse reduces needless kills.
- Keep tags commits green: run full Jest after each task.

**No placeholders:** full code and commands above.
