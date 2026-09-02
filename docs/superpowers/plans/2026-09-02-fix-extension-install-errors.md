# Fix Extension Install Errors: Command Not Found + Schema Validation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix two bugs that prevent the HipCortex VS Code/Antigravity extension from working after install: a stale webpack bundle that causes `command 'hipcortex.queryMemory' not found`, and invalid `description` fields in `languageModelTools` contribution that trigger `Property description is not allowed` JSON schema validation errors.

**Architecture:** Two independent bugs are fixed in sequence. Task 1 repairs the `package.json` schema violations (no build needed, takes effect on extension reload). Task 2 rebuilds the webpack bundle and repackages the `.vsix` so the installed extension contains the current `activate()` function with all 13 command registrations. Both tasks must complete before the repacked `.vsix` is installed.

**Tech Stack:** TypeScript, webpack 5, `@vscode/vsce` (vsix packager), VS Code Extension API ≥ 1.90, JSON Schema draft-07.

## Global Constraints

- VS Code engine minimum: `^1.90.0` — do not lower this constraint.
- Extension entry point: `vscode-extension/dist/extension.js` — do NOT change `main` in `package.json`.
- All paths in this plan are relative to `d:\all_projects\HipCortex\vscode-extension\` unless stated otherwise.
- Never add back a `description` field at the tool-definition level of `languageModelTools`; use `modelDescription` (for LLM) and optionally `userDescription` (for UI display) instead.
- Do not introduce `enabledApiProposals` — the current extension avoids unstable APIs.
- Frequent, atomic commits. One commit per task.
- All `npm` commands run inside `d:\all_projects\HipCortex\vscode-extension\`.

---

## Task 1: Remove invalid `description` from `languageModelTools` in `package.json`

**Background:** VS Code's JSON Schema for the `contributes.languageModelTools` contribution point does **not** allow a top-level `description` field on each tool entry. The allowed fields are `name`, `displayName`, `modelDescription`, `userDescription`, `inputSchema`, and `tags`. All 11 declared tools currently have both `description` (invalid) and `modelDescription` (valid). The fix is to delete the `description` field from each tool entry. `modelDescription` already carries the correct content for the LLM; no text needs to be moved or rewritten.

**Files:**
- Modify: `package.json` (lines 188–407 — the `languageModelTools` array)

**Interfaces:**
- Consumes: existing `package.json` `contributes.languageModelTools` array
- Produces: valid `package.json` with no `description` field at tool-definition level — VS Code's JSON schema validator no longer reports `Property description is not allowed`

---

- [ ] **Step 1: Verify the current violation count**

  Run in PowerShell from `d:\all_projects\HipCortex\vscode-extension\`:

  ```powershell
  $json = Get-Content ".\package.json" -Raw | ConvertFrom-Json
  $tools = $json.contributes.languageModelTools
  foreach ($tool in $tools) {
      $hasDesc = $tool.PSObject.Properties.Name -contains 'description'
      Write-Host "$($tool.name) | has invalid description: $hasDesc"
  }
  ```

  **Expected output** (11 lines, all `True`):
  ```
  hipcortex_search | has invalid description: True
  hipcortex_health | has invalid description: True
  hipcortex_predict | has invalid description: True
  hipcortex_graph_search | has invalid description: True
  hipcortex_topo_ppr | has invalid description: True
  hipcortex_deconstruct | has invalid description: True
  hipcortex_check_edge | has invalid description: True
  hipcortex_can_execute | has invalid description: True
  hipcortex_rollout | has invalid description: True
  hipcortex_system_health | has invalid description: True
  hipcortex_state_diff | has invalid description: True
  ```

---

- [ ] **Step 2: Remove all `description` fields from `languageModelTools` entries in `package.json`**

  Open `package.json`. The `languageModelTools` array starts at line 188 and ends at line 407. For **each** of the 11 tool objects, delete the `"description": "..."` line. The `"modelDescription"` and `"displayName"` lines stay untouched.

  The final shape of every tool entry must look like this (using `hipcortex_search` as the example):

  ```json
  {
      "name": "hipcortex_search",
      "displayName": "HipCortex Memory Search",
      "modelDescription": "Use this FIRST for any recall. Returns unified live_beliefs (memory + world model + hypotheses + coherence). Call before asking the user.",
      "inputSchema": {
          "type": "object",
          "properties": {
              "query": {
                  "type": "string"
              }
          },
          "required": [
              "query"
          ]
      }
  }
  ```

  Repeat for all 11 tools. The `description` values being removed are:

  | Tool | `description` value to remove |
  |---|---|
  | `hipcortex_search` | `"Search HipCortex persistent memory for relevant context. Returns the most relevant memory records matching the query."` |
  | `hipcortex_health` | `"Check HipCortex system health including self-model, world-model, and coherence status."` |
  | `hipcortex_predict` | `"Predict next state using the world-model's Dirichlet-Multinomial transition model."` |
  | `hipcortex_graph_search` | `"PPR-ranked related memory search via CausalTopoGraph. Given a seed memory record ID, returns the most contextually relevant related memories using Personalized PageRank."` |
  | `hipcortex_topo_ppr` | `"Personalized PageRank over the live CausalTopoGraph substrate (symbolic node ids)."` |
  | `hipcortex_deconstruct` | `"Parse free text into causal nodes/edges; optionally apply to live topo graph."` |
  | `hipcortex_check_edge` | `"Check if adding a causal edge would contradict the topo graph."` |
  | `hipcortex_can_execute` | `"SelfModel gate: whether an operation should execute."` |
  | `hipcortex_rollout` | `"Multi-step Dirichlet or goal-shaped MCTS world-model rollout."` |
  | `hipcortex_system_health` | `"Full system health including CalibrationTracker metrics: calibration_score, prediction_error_ewma, consolidation_pressure, epistemic_entropy."` |
  | `hipcortex_state_diff` | `"Causal StateDiff over a tx-log range [from_tx, to_tx]. Returns memory_delta, world_model_delta, and causal attribution paths."` |

---

- [ ] **Step 3: Verify `package.json` is still valid JSON**

  ```powershell
  $content = Get-Content ".\package.json" -Raw
  try {
      $null = $content | ConvertFrom-Json
      Write-Host "✅ package.json is valid JSON"
  } catch {
      Write-Host "❌ JSON parse error: $_"
  }
  ```

  **Expected output:**
  ```
  ✅ package.json is valid JSON
  ```

---

- [ ] **Step 4: Verify zero `description` fields remain at tool-definition level**

  ```powershell
  $json = Get-Content ".\package.json" -Raw | ConvertFrom-Json
  $tools = $json.contributes.languageModelTools
  $violations = @()
  foreach ($tool in $tools) {
      if ($tool.PSObject.Properties.Name -contains 'description') {
          $violations += $tool.name
      }
  }
  if ($violations.Count -eq 0) {
      Write-Host "✅ No invalid description fields remain"
  } else {
      Write-Host "❌ Still has description: $($violations -join ', ')"
  }
  ```

  **Expected output:**
  ```
  ✅ No invalid description fields remain
  ```

---

- [ ] **Step 5: Verify all 11 tools still have `modelDescription`**

  ```powershell
  $json = Get-Content ".\package.json" -Raw | ConvertFrom-Json
  $tools = $json.contributes.languageModelTools
  $missing = @()
  foreach ($tool in $tools) {
      if (-not ($tool.PSObject.Properties.Name -contains 'modelDescription')) {
          $missing += $tool.name
      }
  }
  if ($missing.Count -eq 0) {
      Write-Host "✅ All 11 tools have modelDescription"
  } else {
      Write-Host "❌ Missing modelDescription: $($missing -join ', ')"
  }
  ```

  **Expected output:**
  ```
  ✅ All 11 tools have modelDescription
  ```

---

- [ ] **Step 6: Commit the fix**

  ```powershell
  git add package.json
  git commit -m "fix: remove invalid description field from languageModelTools entries

  VS Code's package.json schema for languageModelTools does not allow a
  top-level 'description' field on tool definitions. The correct field is
  'modelDescription' (already present on all 11 tools). Removes the
  disallowed 'description' field from all 11 entries to resolve the
  'Property description is not allowed' schema validation error."
  ```

---

## Task 2: Rebuild webpack bundle and repackage `.vsix`

**Background:** The installed extension runs from `dist/extension.js`, NOT from `src/extension.ts`. The current `dist/extension.js` was built on **Sep 1, 2026** — it does not contain the `activate()` function body from `src/extension.ts` (last modified Aug 20, 2026), which registers all 13 commands including `hipcortex.queryMemory`. Because `dist/extension.js` is missing all `registerCommand` calls, VS Code loads the extension without any commands, then throws `command 'hipcortex.queryMemory' not found` when the status bar item is clicked (it references that command at `src/extension.ts:1661`). The fix is to run the webpack production build, verify all 13 commands appear in the bundle, and repackage the `.vsix`.

**Files:**
- Modify: `dist/extension.js` (generated by webpack — do not edit manually)
- Produce: `hipcortex-memory-1.3.0.vsix` (matching version in `package.json`) in `vscode-extension/`

**Interfaces:**
- Consumes: `src/extension.ts`, `package.json` (with Task 1 fix applied), `webpack.config.js`
- Produces: `dist/extension.js` containing all 13 `registerCommand` calls; a `.vsix` file ready for installation

---

- [ ] **Step 1: Verify `node_modules` are installed**

  ```powershell
  if (Test-Path ".\node_modules") {
      Write-Host "✅ node_modules present"
  } else {
      Write-Host "node_modules missing — running npm install..."
      npm install
  }
  ```

  If `npm install` runs, wait for it to complete (typically 30–90 seconds). Expected final line: `added N packages` with no errors. If it shows `N vulnerabilities`, that is acceptable and out of scope.

---

- [ ] **Step 2: Run TypeScript compilation check (no emit) to surface type errors before bundling**

  ```powershell
  npx tsc --noEmit -p tsconfig.json
  ```

  **Expected output:** No output (exit code 0 = no errors).

  If errors appear, do **not** fix them in this task — note them for follow-up. Proceed to Step 3 anyway. The `ts-loader` in webpack is configured to continue bundling even with type errors; the command-not-found bug is independent of type errors.

---

- [ ] **Step 3: Run webpack production build**

  ```powershell
  npm run package
  ```

  This executes `webpack --mode production --devtool hidden-source-map` per `package.json` scripts. Expected duration: 10–60 seconds.

  **Expected output ends with:**
  ```
  webpack 5.x.x compiled successfully in NNNNms
  ```

  `compiled with N warning(s)` is acceptable. `compiled with N error(s)` is a hard stop — report the full error text and do not proceed.

---

- [ ] **Step 4: Verify `dist/extension.js` now contains all 13 command registrations**

  ```powershell
  $bundle = Get-Content ".\dist\extension.js" -Raw
  $commands = @(
      "hipcortex.queryMemory",
      "hipcortex.addMemory",
      "hipcortex.testExtension",
      "hipcortex.restartServer",
      "hipcortex.systemHealth",
      "hipcortex.stateDiff",
      "hipcortex.cognitiveHealth",
      "hipcortex.cognitiveSnapshot",
      "hipcortex.twinCreate",
      "hipcortex.twinStep",
      "hipcortex.twinRollout",
      "hipcortex.twinGet",
      "hipcortex.experienceTiers"
  )
  $missing = @()
  foreach ($cmd in $commands) {
      if ($bundle -notmatch [regex]::Escape($cmd)) {
          $missing += $cmd
      }
  }
  if ($missing.Count -eq 0) {
      Write-Host "✅ All 13 commands present in dist/extension.js"
  } else {
      Write-Host "❌ Missing from bundle: $($missing -join ', ')"
  }
  ```

  **Expected output:**
  ```
  ✅ All 13 commands present in dist/extension.js
  ```

  If any command is missing, the webpack build did not pick up `src/extension.ts` correctly — check `webpack.config.js` for the entry point and run Step 3 again.

---

- [ ] **Step 5: Package the `.vsix`**

  ```powershell
  npx --yes @vscode/vsce package --no-dependencies
  ```

  **Expected output ends with:**
  ```
  DONE Packaged: d:\all_projects\HipCortex\vscode-extension\hipcortex-memory-1.3.0.vsix (N files, N.NNMb)
  ```

  Warnings about missing `server/` binary files (the bundled Rust server) are acceptable for a local dev install. The `.vsix` is still installable.

  Confirm the file exists:

  ```powershell
  Get-Item ".\hipcortex-memory-*.vsix" | Select-Object Name, Length, LastWriteTime
  ```

  **Expected output:** One row with a `.vsix` file dated Sep 2, 2026 and a non-zero size (typically 1–10 MB without binaries, 20–80 MB with binaries).

---

- [ ] **Step 6: Install the new `.vsix` in VS Code / Antigravity**

  **Option A — VS Code GUI (interactive):**
  1. Open the Extensions panel (`Ctrl+Shift+X`)
  2. Click the `···` (three-dot) menu at the top-right → "Install from VSIX..."
  3. Navigate to and select `hipcortex-memory-1.3.0.vsix`
  4. Click "Reload Required" banner when it appears

  **Option B — CLI (if VS Code CLI is on PATH):**
  ```powershell
  code --install-extension ".\hipcortex-memory-1.3.0.vsix"
  ```
  Then reload: `Ctrl+Shift+P` → type "Developer: Reload Window" → Enter.

  After reload, the extension activates. The Output panel → "HipCortex Server" channel should appear and print:
  ```
  🧠 HipCortex Memory Extension v0.3.3 active
  🔧 Registering chat participant: hipcortex
  ```
  (The version string `v0.3.3` is hard-coded in the `activate()` log; this is unrelated to the package version `1.3.0`.)

---

- [ ] **Step 7: Smoke-test all commands via Command Palette**

  Open the Command Palette (`Ctrl+Shift+P`), type `HipCortex`, and verify every one of these entries appears:

  ```
  HipCortex: Query Memory Records
  HipCortex: Add Memory Record
  HipCortex: Test HipCortex Extension
  HipCortex: Restart HipCortex Server
  HipCortex: Show System Health
  HipCortex: Compute State Diff
  HipCortex: Cognitive Health Status
  HipCortex: Show Cognitive Snapshot
  HipCortex: Create DigitalTwin
  HipCortex: DigitalTwin: Step
  HipCortex: DigitalTwin: Rollout
  HipCortex: DigitalTwin: Show State
  HipCortex: Show Experience Tier Stats
  ```

  Select "HipCortex: Query Memory Records". It must either:
  - Show a QuickPick list of memory records (server running), OR
  - Show an error message like `Error: connect ECONNREFUSED 127.0.0.1:3030` (server not running)

  **The error must NOT be** `command 'hipcortex.queryMemory' not found`. That error means the bundle is still stale.

---

- [ ] **Step 8: Verify the schema validation warning is gone**

  1. Open `package.json` in the editor.
  2. Open the Problems panel (`Ctrl+Shift+M`).
  3. Filter by file to `package.json`.

  **Expected result:** Zero errors or warnings with the text `Property description is not allowed`.

  If any remain after Task 1's fix, open the Problems panel entry to see the exact line number — it may point to a different section of `package.json` not covered by this plan.

---

- [ ] **Step 9: Commit**

  ```powershell
  git add dist/extension.js
  git commit -m "fix: rebuild webpack bundle to register all 13 commands

  The dist/extension.js bundle was stale (built Sep 1 2026, before the
  activate() function's command registrations were written). Running
  'npm run package' regenerates the bundle from src/extension.ts,
  which contains registerCommand() calls for all 13 hipcortex.* commands.

  Resolves: command 'hipcortex.queryMemory' not found on extension install"
  ```

---

## Final Verification Script

Run this after completing both tasks to confirm all success conditions at once:

```powershell
# Run from d:\all_projects\HipCortex\vscode-extension\
Write-Host "=== HipCortex Extension Fix Verification ==="

# 1. JSON validity
try {
    $null = (Get-Content .\package.json -Raw | ConvertFrom-Json)
    Write-Host "✅ package.json is valid JSON"
} catch { Write-Host "❌ package.json parse error: $_" }

# 2. No schema violations
$json = Get-Content .\package.json -Raw | ConvertFrom-Json
$bad = $json.contributes.languageModelTools | Where-Object { $_.PSObject.Properties.Name -contains 'description' }
if ($bad.Count -eq 0) { Write-Host "✅ No invalid description fields in languageModelTools" }
else { Write-Host "❌ Still have description: $($bad.name -join ', ')" }

# 3. All modelDescriptions present
$missing = $json.contributes.languageModelTools | Where-Object { -not ($_.PSObject.Properties.Name -contains 'modelDescription') }
if ($missing.Count -eq 0) { Write-Host "✅ All 11 tools have modelDescription" }
else { Write-Host "❌ Missing modelDescription: $($missing.name -join ', ')" }

# 4. All 13 commands in bundle
$b = Get-Content .\dist\extension.js -Raw
$cmds = "queryMemory","addMemory","testExtension","restartServer","systemHealth","stateDiff","cognitiveHealth","cognitiveSnapshot","twinCreate","twinStep","twinRollout","twinGet","experienceTiers"
$missingCmds = $cmds | Where-Object { $b -notmatch "hipcortex\.$_" }
if ($missingCmds.Count -eq 0) { Write-Host "✅ All 13 commands present in dist/extension.js" }
else { Write-Host "❌ Missing from bundle: $($missingCmds | ForEach-Object { "hipcortex.$_" } | Join-String -Separator ', ')" }

# 5. .vsix exists
$vsix = Get-Item .\hipcortex-memory-*.vsix -ErrorAction SilentlyContinue
if ($vsix) { Write-Host "✅ .vsix exists: $($vsix.Name) ($([math]::Round($vsix.Length/1MB, 1)) MB)" }
else { Write-Host "❌ No .vsix found — run Step 5 of Task 2" }

Write-Host "=== Done ==="
```

**Expected output (all green):**
```
=== HipCortex Extension Fix Verification ===
✅ package.json is valid JSON
✅ No invalid description fields in languageModelTools
✅ All 11 tools have modelDescription
✅ All 13 commands present in dist/extension.js
✅ .vsix exists: hipcortex-memory-1.3.0.vsix (X.X MB)
=== Done ===
```
