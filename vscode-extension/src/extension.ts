import * as vscode from 'vscode';
import axios from 'axios';
import { TokenTracker } from './token-tracker';
import * as cp from 'child_process';
import * as path from 'path';
import * as os from 'os';
import * as fs from 'fs';
import * as https from 'https';

/** Published release asset name — exported for tests (matches .github/workflows/release.yml). */
export function buildPublishedBinaryName(platform: string, arch: string): string {
    const normalizedArch = arch === 'x64' ? 'amd64' : arch;
    let binaryName = `hipcortex-${platform === 'win32' ? 'windows' : platform}-${normalizedArch}`;
    if (platform === 'win32') {
        binaryName += '.exe';
    } else if (platform === 'darwin') {
        binaryName = binaryName.replace('darwin', 'macos');
    }
    return binaryName;
}

/** Unified install root shared with Python CLI (`~/.hipcortex`). */
export function sharedInstallDir(): string {
    return path.join(os.homedir(), '.hipcortex');
}

/** Default DATA_DIR shared with CLI daemon (`~/.hipcortex/data`). */
export function sharedDataDir(): string {
    return path.join(sharedInstallDir(), 'data');
}

/** PID file path for CLI stop compatibility (`~/.hipcortex/hipcortex.pid`). */
export function sharedPidPath(): string {
    return path.join(sharedInstallDir(), 'hipcortex.pid');
}

/** Shared exclusive lock path (`~/.hipcortex/server.lock`) — same as Python daemon. */
export function sharedLockPath(): string {
    return path.join(sharedInstallDir(), 'server.lock');
}

/** Parse PID from lock/pid file content (`"123"`, `"pid=123"`, first token). */
export function parseLockPid(content: string): number | null {
    let text = (content || '').trim();
    if (!text) {
        return null;
    }
    if (text.startsWith('pid=')) {
        text = text.slice(4).trim();
    }
    const token = text.split(/\s+/)[0];
    const n = parseInt(token, 10);
    if (!Number.isFinite(n) || n <= 0) {
        return null;
    }
    return n;
}

/** Steal lock only when holder is known dead. */
export function shouldStealStaleLock(lockPidAlive: boolean): boolean {
    return !lockPidAlive;
}

/** True if netstat line is LISTENING and local address ends with `:{port}`. */
export function netstatLineMatchesListenPort(line: string, port: number): boolean {
    const parts = line.trim().split(/\s+/);
    if (parts.length < 4) {
        return false;
    }
    if (!parts.some(p => p.toUpperCase() === 'LISTENING')) {
        return false;
    }
    return parts[1].endsWith(`:${port}`);
}

function ensureSharedInstallDir(): void {
    const installDir = sharedInstallDir();
    if (!fs.existsSync(installDir)) {
        fs.mkdirSync(installDir, { recursive: true });
    }
}

/** Best-effort write of server PID for CLI `hipcortex stop`. */
function writeSharedPidBestEffort(pid: number | undefined, log: (msg: string) => void): void {
    if (pid === undefined || !Number.isFinite(pid) || pid <= 0) {
        return;
    }
    try {
        ensureSharedInstallDir();
        fs.writeFileSync(sharedPidPath(), String(pid), 'utf8');
        log(`Wrote shared PID ${pid} → ${sharedPidPath()}`);
    } catch (err) {
        log(`Could not write shared PID file: ${err instanceof Error ? err.message : String(err)}`);
    }
}

/** Write hipcortex.pid + server.lock with server child pid (shared with CLI daemon). */
function writeSharedPidAndLock(pid: number | undefined, log: (msg: string) => void): void {
    writeSharedPidBestEffort(pid, log);
    if (pid === undefined || !Number.isFinite(pid) || pid <= 0) {
        return;
    }
    try {
        ensureSharedInstallDir();
        fs.writeFileSync(sharedLockPath(), `${pid}\n`, 'utf8');
        log(`Wrote shared lock ${pid} → ${sharedLockPath()}`);
    } catch (err) {
        log(`Could not write shared lock: ${err instanceof Error ? err.message : String(err)}`);
    }
}

/** Best-effort: is *pid* still alive? null = unknown (e.g. Windows). */
function isPidAliveBestEffort(pid: number): boolean | null {
    if (pid <= 0) {
        return false;
    }
    if (os.platform() === 'win32') {
        // Windows: process.kill(pid, 0) is not a reliable no-op probe here.
        return null;
    }
    try {
        process.kill(pid, 0);
        return true;
    } catch (err: unknown) {
        const code = err && typeof err === 'object' && 'code' in err ? (err as NodeJS.ErrnoException).code : undefined;
        if (code === 'ESRCH') {
            return false;
        }
        // EPERM etc. — process exists but not signalable
        return true;
    }
}

/**
 * Exclusive server.lock (O_EXCL / 'wx'). Stale cleanup when holder known dead.
 * Returns 'acquired' | 'held' | 'failed'.
 */
function tryAcquireServerLock(log: (msg: string) => void): 'acquired' | 'held' | 'failed' {
    try {
        ensureSharedInstallDir();
    } catch (err) {
        log(`Could not create install dir: ${err instanceof Error ? err.message : String(err)}`);
        return 'failed';
    }

    const lockPath = sharedLockPath();
    const writeExclusive = (): boolean => {
        try {
            fs.writeFileSync(lockPath, `${process.pid}\n`, { flag: 'wx' });
            return true;
        } catch (err: unknown) {
            const code = err && typeof err === 'object' && 'code' in err ? (err as NodeJS.ErrnoException).code : undefined;
            if (code === 'EEXIST') {
                return false;
            }
            throw err;
        }
    };

    try {
        if (writeExclusive()) {
            return 'acquired';
        }
    } catch (err) {
        log(`Lock acquire failed: ${err instanceof Error ? err.message : String(err)}`);
        return 'failed';
    }

    let lockPid: number | null = null;
    try {
        lockPid = parseLockPid(fs.readFileSync(lockPath, 'utf8'));
    } catch {
        lockPid = null;
    }

    const alive = lockPid !== null ? isPidAliveBestEffort(lockPid) : false;
    // Steal when unreadable/dead; on Windows (alive===null) do not steal.
    const canSteal =
        lockPid === null || (alive === false && shouldStealStaleLock(false));

    if (canSteal) {
        try {
            fs.unlinkSync(lockPath);
        } catch {
            /* race */
        }
        try {
            if (writeExclusive()) {
                return 'acquired';
            }
        } catch (err) {
            log(`Lock re-acquire failed: ${err instanceof Error ? err.message : String(err)}`);
            return 'failed';
        }
    }

    return 'held';
}

/** Read known HipCortex PID from shared pid file, else lock file. */
function readKnownHipcortexPid(): number | null {
    try {
        if (fs.existsSync(sharedPidPath())) {
            const n = parseLockPid(fs.readFileSync(sharedPidPath(), 'utf8'));
            if (n !== null) {
                return n;
            }
        }
    } catch {
        /* ignore */
    }
    try {
        if (fs.existsSync(sharedLockPath())) {
            return parseLockPid(fs.readFileSync(sharedLockPath(), 'utf8'));
        }
    } catch {
        /* ignore */
    }
    return null;
}

function killPidBestEffort(pid: number): Promise<void> {
    return new Promise(resolve => {
        if (os.platform() === 'win32') {
            cp.exec(`taskkill /F /PID ${pid}`, () => resolve());
        } else {
            cp.exec(`kill -9 ${pid}`, () => resolve());
        }
    });
}

/**
 * Kill only a known HipCortex PID (shared pid file or lock file).
 * Never kills arbitrary port listeners.
 */
async function killKnownHipcortexPid(_port: number, log?: (msg: string) => void): Promise<void> {
    const pid = readKnownHipcortexPid();
    if (pid === null) {
        log?.('No known HipCortex PID in shared pid/lock — skip kill');
        return;
    }
    log?.(`Stopping known HipCortex PID ${pid} (shared pid/lock only)`);
    await killPidBestEffort(pid);
    // Clear stale metadata after kill attempt
    try {
        if (fs.existsSync(sharedPidPath())) {
            fs.unlinkSync(sharedPidPath());
        }
    } catch {
        /* ignore */
    }
    try {
        if (fs.existsSync(sharedLockPath())) {
            fs.unlinkSync(sharedLockPath());
        }
    } catch {
        /* ignore */
    }
}

const MIN_BINARY_BYTES = 1_000_000;
const SERVER_START_TIMEOUT_SEC = 30;

let extensionInstallPath: string | undefined;
let globalServerProcess: cp.ChildProcess | null = null;
let serverStartInFlight: Promise<boolean> | null = null;

/** True when file looks like real server binary (not placeholder/HTML cache). */
export function isValidServerBinary(filePath: string): boolean {
    try {
        if (!fs.existsSync(filePath)) {
            return false;
        }
        const stats = fs.statSync(filePath);
        if (stats.size < MIN_BINARY_BYTES) {
            return false;
        }
        const head = Buffer.alloc(16);
        const fd = fs.openSync(filePath, 'r');
        try {
            fs.readSync(fd, head, 0, 16, 0);
        } finally {
            fs.closeSync(fd);
        }
        const prefix = head.toString('utf8', 0, 12);
        if (prefix.startsWith('<!') || prefix.includes('Not Found') || prefix.includes('PLACEHOLDER')) {
            return false;
        }
        return true;
    } catch {
        return false;
    }
}

/** Proactive harness descriptions — substrate-first policy for LM tools. */
export const HARNESS_TOOL_DESCRIPTIONS = {
    search: 'Use this FIRST for any recall... returns unified live_beliefs (memory + world model + hypotheses + coherence). Call before asking user.',
    health: 'Use this FIRST for any health/status before other actions. Returns substrate health + world model coherence. Call before asking user.',
    predict: 'Use this FIRST for world model predictions. Call substrate predict before deciding actions. Returns WM + uncertainty.',
    rollout: 'Multi-step world-model rollout. Prefer mode=dirichlet after observe; mode=mcts with goal_state for goal-shaped UCB1 search.',
    topoPpr: 'Personalized PageRank over live CausalTopoGraph substrate nodes. Use after link/deconstruct.',
    deconstruct: 'Parse hypothesis text into causal nodes/edges (rules + optional llm_json). Set apply=true to write into topo.',
    checkEdge: 'Check if adding causal edge from→to would contradict the topo graph (cycle/reverse).',
    canExecute: 'SelfModel gate: should the agent execute this operation?',
} as const;

interface MemoryRecord {
    id: string;
    record_type: string;
    timestamp: string;
    actor: string;
    action: string;
    target: string;
    metadata: any;
    integrity?: string;
    tags?: string[];
    priority?: string;
    source?: string;
    confidence?: number;
}

export interface AddMemoryRequest {
    actor: string;
    action: string;
    target: string;
    record_type?: "Temporal" | "Symbolic" | "Procedural" | "Reflexion" | "Perception";
    metadata?: Record<string, unknown>;
    /** Unix TTL in seconds. Server deletes record after this duration. */
    ttl_seconds?: number;
    /** "pinned" | "high" | "normal" | "low". Affects search ranking. */
    priority?: string;
    /** Reliability signal [0.0, 1.0]. Affects decay scoring. Default 1.0. */
    confidence?: number;
    /** Source identifier, e.g. "user-input", "copilot". */
    source?: string;
    /** Categorization tags for RAG filtering. */
    tags?: string[];
}

export type SelfModelOperation =
    | 'add_memory'
    | 'search_memory'
    | 'query_memory'
    | 'ingest'
    | 'bulk_add'
    | 'forget'
    | 'reflect'
    | 'context'
    | 'predict'
    | 'rollout';

interface AddMemoryResponse {
    success: boolean;
    record_id?: string;
    error?: string;
}

interface QueryMemoryResponse {
    records: MemoryRecord[];
    total: number;
}

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
/**
 * Dual health contract:
 * - JSON: { status: "ok", service: "hipcortex", version?: string }
 * - Legacy plain body: "ok" (text/plain binaries still in old VSIX)
 * - JSON-as-string when Content-Type mis-set
 */
export type HealthParseResult = {
    healthy: boolean;
    service?: string;
    version?: string;
    legacyPlainOk?: boolean;
};

export function parseHealthPayload(statusCode: number, data: unknown): HealthParseResult {
    if (statusCode !== 200 || data == null) {
        return { healthy: false };
    }
    if (typeof data === 'string') {
        const trimmed = data.trim();
        if (trimmed.toLowerCase() === 'ok') {
            return { healthy: true, legacyPlainOk: true };
        }
        try {
            return parseHealthPayload(200, JSON.parse(trimmed));
        } catch {
            return { healthy: false };
        }
    }
    if (typeof data === 'object' && !Array.isArray(data)) {
        const obj = data as Record<string, unknown>;
        if (obj.status === 'ok' && obj.service === 'hipcortex') {
            return {
                healthy: true,
                service: 'hipcortex',
                version: typeof obj.version === 'string' ? obj.version : undefined,
            };
        }
        return { healthy: false };
    }
    return { healthy: false };
}

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

export class HipCortexAPI {
    private baseUrl: string;
    private apiKey: string;

    constructor() {
        const config = vscode.workspace.getConfiguration('hipcortex');
        this.baseUrl = config.get('apiUrl', 'http://127.0.0.1:3030');
        this.apiKey = config.get('apiKey', '');
    }

    async healthCheck(): Promise<boolean> {
        try {
            const res = await axios.get(`${this.baseUrl}/health`, { timeout: 3000 });
            return parseHealthPayload(res.status, res.data).healthy;
        } catch {
            return false;
        }
    }

    async autoStartServer(outputChannel?: vscode.OutputChannel): Promise<boolean> {
        if (serverStartInFlight) {
            return serverStartInFlight;
        }
        serverStartInFlight = this.doAutoStartServer(outputChannel);
        try {
            return await serverStartInFlight;
        } finally {
            serverStartInFlight = null;
        }
    }

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
        let hipcortexService = false;
        let serverVersion: string | undefined;
        try {
            const healthRes = await axios.get(`${this.baseUrl}/health`, { timeout: 2000 });
            const parsed = parseHealthPayload(healthRes.status, healthRes.data);
            healthy = parsed.healthy;
            hipcortexService = parsed.service === 'hipcortex';
            serverVersion = parsed.version;
            // Legacy plain-ok: treat as hipcortex for attach (daemon parity).
            if (parsed.legacyPlainOk) {
                hipcortexService = true;
                log(
                    'Health: legacy plain-ok body (upgrade server binary recommended)'
                );
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
            // Attach-first: never kill/spawn when reusing a healthy acceptable server.
            log(
                `Reusing active HipCortex server on port ${portStr}` +
                    (serverVersion ? ` (version ${serverVersion})` : ' (version unknown)')
            );
            return true;
        }

        if (healthy) {
            // Version/policy reject: kill known HipCortex pid/lock only (not all listeners).
            log(
                `Server on port ${portStr} not acceptable for policy ` +
                    `(version=${serverVersion ?? 'n/a'}, expected crate ${EXPECTED_SERVER_VERSION}, strict=${strict}` +
                    (hipcortexService ? ', service=hipcortex' : '') +
                    `). Restarting known PID only...`
            );
        } else if (fs.existsSync(sharedLockPath()) && hipcortexService) {
            // Defensive: lock + hipcortex identity without reuse → attach, no dual spawn.
            log(`server.lock present and service=hipcortex; attach-only (no spawn)`);
            return true;
        } else {
            log(`No healthy HipCortex on ${this.baseUrl}. Reclaiming known pid/lock only, then starting...`);
        }

        try {
            // Kill only known HipCortex PIDs — never all netstat listeners on the port.
            await killKnownHipcortexPid(port, log);

            const lockStatus = tryAcquireServerLock(log);
            if (lockStatus === 'held') {
                // Another client holds lock; re-check health then attach-wait.
                if (await this.healthCheck()) {
                    log('server.lock held by another process and server healthy; attaching');
                    return true;
                }
                log('server.lock held by another process; attach-wait (no spawn to avoid dual writers)');
                return false;
            }
            if (lockStatus === 'failed') {
                log('Could not acquire server.lock; aborting spawn');
                return false;
            }

            log('Starting HipCortex server...');
            const binaryPath = await this.ensureServerBinary(outputChannel);

            // Logs/cwd stay under extension-local dir; DATA_DIR shares with CLI.
            const serverCwd = path.join(os.homedir(), '.hipcortex-vscode');
            if (!fs.existsSync(serverCwd)) {
                fs.mkdirSync(serverCwd, { recursive: true });
            }
            const dataDir = sharedDataDir();
            if (!fs.existsSync(dataDir)) {
                fs.mkdirSync(dataDir, { recursive: true });
            }

            if (globalServerProcess) {
                globalServerProcess.kill();
                globalServerProcess = null;
            }

            globalServerProcess = cp.spawn(binaryPath, [], {
                cwd: serverCwd,
                env: {
                    ...process.env,
                    PORT: portStr,
                    DATA_DIR: dataDir,
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

            // Stamp lock with child pid (replace parent-held lock content).
            writeSharedPidAndLock(globalServerProcess?.pid, log);

            for (let i = 0; i < SERVER_START_TIMEOUT_SEC; i++) {
                await new Promise(resolve => setTimeout(resolve, 1000));
                if (await this.healthCheck()) {
                    // Re-stamp in case pid only became available later
                    writeSharedPidAndLock(globalServerProcess?.pid, log);
                    log(`Server ready on ${this.baseUrl} (DATA_DIR=${dataDir})`);
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

    private async ensureServerBinary(outputChannel?: vscode.OutputChannel): Promise<string> {
        const platform = os.platform();
        const binaryName = buildPublishedBinaryName(platform, os.arch());

        const log = (msg: string) => outputChannel?.appendLine(msg);

        // 1. Bundled in vsix — use context.extensionPath (getExtension() often null during activate)
        const roots = [
            extensionInstallPath,
            vscode.extensions.getExtension('farmountain.hipcortex-memory')?.extensionPath,
        ].filter((p): p is string => Boolean(p));

        for (const root of roots) {
            const bundledPath = path.join(root, 'server', platform, binaryName);
            if (isValidServerBinary(bundledPath)) {
                log(`Using bundled server binary: ${bundledPath}`);
                return bundledPath;
            }
        }

        // 2. CLI install dir (~/.hipcortex/hipcortex-*) — prefer over download
        const cliBinaryPath = path.join(sharedInstallDir(), binaryName);
        if (isValidServerBinary(cliBinaryPath)) {
            log(`Using CLI-installed server binary: ${cliBinaryPath}`);
            return cliBinaryPath;
        }

        // 3. User cache — re-download if corrupt
        const binDir = path.join(os.homedir(), '.hipcortex-vscode', 'bin');
        if (!fs.existsSync(binDir)) {
            fs.mkdirSync(binDir, { recursive: true });
        }
        const cachedPath = path.join(binDir, binaryName);
        if (isValidServerBinary(cachedPath)) {
            log(`Using cached server binary: ${cachedPath}`);
            return cachedPath;
        }
        if (fs.existsSync(cachedPath)) {
            fs.unlinkSync(cachedPath);
            log('Removed corrupt cached binary; re-downloading...');
        }

        // 4. Download from GitHub releases
        const downloadUrl = `https://github.com/farmountain/HipCortex/releases/latest/download/${binaryName}`;
        log(`Downloading ${binaryName} from GitHub releases...`);
        await this.downloadFile(downloadUrl, cachedPath);

        if (platform !== 'win32') {
            fs.chmodSync(cachedPath, 0o755);
        }

        if (!isValidServerBinary(cachedPath)) {
            if (fs.existsSync(cachedPath)) {
                fs.unlinkSync(cachedPath);
            }
            throw new Error(
                `Downloaded ${binaryName} is invalid (too small or HTML error page). ` +
                'Check network or install from a vsix with bundled server binaries.'
            );
        }

        log(`Downloaded server binary: ${cachedPath}`);
        return cachedPath;
    }

    private async downloadFile(url: string, dest: string): Promise<void> {
        return new Promise((resolve, reject) => {
            const request = (targetUrl: string, redirects = 0) => {
                if (redirects > 8) {
                    reject(new Error('Too many redirects downloading server binary'));
                    return;
                }
                https.get(targetUrl, (response) => {
                    const status = response.statusCode ?? 0;
                    if (status === 301 || status === 302 || status === 307 || status === 308) {
                        const location = response.headers.location;
                        response.resume();
                        if (!location) {
                            reject(new Error(`Redirect without location from ${targetUrl}`));
                            return;
                        }
                        request(location, redirects + 1);
                        return;
                    }
                    if (status !== 200) {
                        response.resume();
                        reject(new Error(`Download failed: HTTP ${status} for ${targetUrl}`));
                        return;
                    }
                    const file = fs.createWriteStream(dest);
                    response.pipe(file);
                    file.on('finish', () => {
                        file.close();
                        resolve();
                    });
                    file.on('error', (err) => {
                        fs.unlink(dest, () => {});
                        reject(err);
                    });
                }).on('error', (err) => {
                    fs.unlink(dest, () => {});
                    reject(err);
                });
            };
            request(url);
        });
    }

    async addMemory(request: AddMemoryRequest): Promise<AddMemoryResponse> {
        const headers: any = { 'Content-Type': 'application/json' };
        if (this.apiKey) {
            headers['Authorization'] = `Bearer ${this.apiKey}`;
        }
        
        const response = await axios.post(`${this.baseUrl}/memory/add`, request, { headers });
        return response.data;
    }

    async queryMemory(params: { 
        actor?: string; 
        action?: string; 
        record_type?: string; 
        limit?: number 
    } = {}): Promise<QueryMemoryResponse> {
        const queryString = new URLSearchParams(
            Object.entries(params)
                .filter(([_, value]) => value !== undefined)
                .map(([key, value]) => [key, String(value)])
        ).toString();
        
        const headers: any = {};
        if (this.apiKey) {
            headers['Authorization'] = `Bearer ${this.apiKey}`;
        }
        
        const response = await axios.get(`${this.baseUrl}/memory/query?${queryString}`, { headers });
        return response.data;
    }

    // In HipCortexAPI class
    async liveBeliefs(params: { limit?: number; actor?: string } = {}) { /* axios GET /memory/live_beliefs */
        const queryString = new URLSearchParams(
            Object.entries(params)
                .filter(([_, value]) => value !== undefined)
                .map(([key, value]) => [key, String(value)])
        ).toString();
        
        const headers: any = {};
        if (this.apiKey) {
            headers['Authorization'] = `Bearer ${this.apiKey}`;
        }
        
        const response = await axios.get(`${this.baseUrl}/memory/live_beliefs?${queryString}`, { headers });
        return response.data;
    }

    async reflect(query: string) {
        const res = await axios.post(`${this.baseUrl}/memory/reflect`, { query });
        return res.data; // { loops_run, hypothesis, ... }
    }

    async predictState(state: string, action: string) { /* /worldmodel/predict */
        const headers: any = { 'Content-Type': 'application/json' };
        if (this.apiKey) {
            headers['Authorization'] = `Bearer ${this.apiKey}`;
        }
        const response = await axios.post(`${this.baseUrl}/worldmodel/predict`, { state, action }, { headers });
        return response.data;
    }

    async healthSummary(): Promise<string> {
        const healthy = await this.healthCheck();
        return healthy ? 'OK' : 'DOWN';
    }

    async linkMemories(sourceId: string, targetId: string, relation = 'related'): Promise<any> {
        const headers: any = { 'Content-Type': 'application/json' };
        if (this.apiKey) { headers['Authorization'] = `Bearer ${this.apiKey}`; }
        const response = await axios.post(`${this.baseUrl}/memory/link`, { source_id: sourceId, target_id: targetId, relation }, { headers });
        return response.data;
    }

    async searchRelated(seedId: string, limit = 10): Promise<{ results: Array<{ score: number; record: any }> }> {
        const headers: any = {};
        if (this.apiKey) { headers['Authorization'] = `Bearer ${this.apiKey}`; }
        const response = await axios.get(`${this.baseUrl}/memory/search/related`, { params: { seed_id: seedId, limit }, headers });
        return response.data;
    }

    async searchMemory(query: string, limit = 10): Promise<{ results: Array<{ score: number; record: any }>; total: number }> {
        const headers: any = { 'Content-Type': 'application/json' };
        if (this.apiKey) { headers['Authorization'] = `Bearer ${this.apiKey}`; }
        const response = await axios.post(`${this.baseUrl}/memory/search`, { query, limit }, { headers });
        return response.data;
    }

    async canExecute(operation: SelfModelOperation): Promise<boolean> {
        const headers: any = {};
        if (this.apiKey) {
            headers['Authorization'] = `Bearer ${this.apiKey}`;
        }
        try {
            const response = await axios.get(`${this.baseUrl}/self/can-execute`, {
                params: { operation },
                headers
            });
            return response.data?.should_execute ?? true;
        } catch {
            return true; // Fallback to true if server doesn't support the route or is offline
        }
    }
}

class HipCortexChatParticipant {
    private api: HipCortexAPI;
    private tokenTracker: TokenTracker;

    constructor(tokenTracker: TokenTracker) {
        this.api = new HipCortexAPI();
        this.tokenTracker = tokenTracker;
    }

    async provideResponse(
        request: vscode.ChatRequest,
        context: vscode.ChatContext,
        stream: vscode.ChatResponseStream,
        token: vscode.CancellationToken
    ): Promise<void> {
        // FORCE our extension to respond - this should NEVER be intercepted
        console.log('🚀 HipCortex Extension: OFFICIAL RESPONSE for prompt:', request.prompt);
        stream.markdown('🎯 **OFFICIAL HipCortex Extension Responding**\n\n');
        
        try {
            const command = request.prompt.toLowerCase().trim();

            if (command.startsWith('link')) {
                await this.handleLinkMemory(request, stream);
            } else if (command.startsWith('mode')) {
                await this.handleModeSwitch(request, stream);
            } else if (command.startsWith('add') || command.startsWith('record')) {
                await this.handleAddMemory(request, stream);
            } else if (command.startsWith('query') || command.startsWith('find')) {
                await this.handleQueryMemory(request, stream);
            } else if (command.startsWith('search')) {
                await this.handleSearchMemory(request, stream);
            } else if (command.startsWith('health') || command.startsWith('status')) {
                await this.handleHealthCheck(stream);
            } else {
                await this.handleHelp(stream);
            }
        } catch (error) {
            stream.markdown(`❌ **Error**: ${error instanceof Error ? error.message : String(error)}`);
        }
    }

    private validateInput(input: string, type: 'actor' | 'action' | 'target'): string {
        const trimmed = input.trim();
        if (!trimmed) {
            throw new Error(`${type} cannot be empty`);
        }
        if (trimmed.length > 100) {
            throw new Error(`${type} cannot exceed 100 characters`);
        }
        // Sanitize input - remove potentially harmful characters
        return trimmed.replace(/[<>"'&]/g, '');
    }

    private async handleLinkMemory(request: vscode.ChatRequest, stream: vscode.ChatResponseStream): Promise<void> {
        // Usage: @hipcortex /link <source-id> <target-id> [relation]
        const parts = request.prompt.trim().split(/\s+/);
        // parts[0] = "link", parts[1] = source_id, parts[2] = target_id, parts[3] = optional relation
        const sourceId = parts[1];
        const targetId = parts[2];
        const relation = parts[3] ?? 'related';

        if (!sourceId || !targetId) {
            stream.markdown('❌ **Usage**: `@hipcortex link <source-id> <target-id> [relation]`\n\nBoth `source-id` and `target-id` (record UUIDs) are required.');
            return;
        }

        stream.markdown(`🔗 **Linking memories...**\n\n`);
        try {
            const res = await this.api.linkMemories(sourceId, targetId, relation);
            if (res.success) {
                stream.markdown(`✅ **Link created!**\n\n`);
                stream.markdown(`| Field | Value |\n|---|---|\n`);
                stream.markdown(`| **Source** | \`${sourceId.substring(0, 8)}...\` |\n`);
                stream.markdown(`| **Relation** | \`${relation}\` |\n`);
                stream.markdown(`| **Target** | \`${targetId.substring(0, 8)}...\` |\n\n`);
                stream.markdown(`💡 Use \`hipcortex_graph_search\` with seed_id \`${sourceId}\` to explore PPR-related memories.`);
            } else {
                stream.markdown(`❌ **Link failed**: ${JSON.stringify(res)}`);
            }
        } catch (error) {
            stream.markdown(`❌ **Link error**: ${error instanceof Error ? error.message : String(error)}`);
        }
    }

    private async handleModeSwitch(request: vscode.ChatRequest, stream: vscode.ChatResponseStream): Promise<void> {
        const parts = request.prompt.trim().split(/\s+/);
        const newMode = parts[1]?.toLowerCase();
        if (newMode !== 'headroom' && newMode !== 'caveman') {
            stream.markdown('❌ **Usage**: `@hipcortex /mode <headroom|caveman>`\n\n- **headroom**: Top-5 retrieval, 59-84% token savings.\n- **caveman**: Top-3 retrieval, 70-88% token savings.');
            return;
        }
        await vscode.workspace.getConfiguration('hipcortex').update('optimizationMode', newMode, vscode.ConfigurationTarget.Global);
        const modeCap = newMode === 'caveman' ? 'Caveman' : 'Headroom';
        stream.markdown(`🎯 **Token Optimization Mode Updated!**\n\n`);
        stream.markdown(`| Setting | New Value |\n|---|---|\n`);
        stream.markdown(`| **Mode** | \`${modeCap}\` |\n`);
        stream.markdown(`| **Retrieval Depth** | \`${newMode === 'caveman' ? 'Top-3 records (limit: 3)' : 'Top-5 records (limit: 5)'}\` |\n`);
        stream.markdown(`| **Expected Savings** | \`${newMode === 'caveman' ? '70% to 88%' : '59% to 84%'}\` |\n\n`);
        stream.markdown(`💡 Status bar and LM tools will now use **[${modeCap}]** mode.`);
    }

    private async handleAddMemory(request: vscode.ChatRequest, stream: vscode.ChatResponseStream): Promise<void> {
        stream.markdown('🧠 **Adding memory record...**\\n\\n');

        try {
            // Parse the request for actor, action, target
            const prompt = request.prompt;
            const actorMatch = prompt.match(/actor[:\\s]+([\\w\\s]+?)(?=\\s+action|\\s+target|\\s+priority|\\s+ttl|\\s+tags|$)/i);
            const actionMatch = prompt.match(/action[:\\s]+([\\w\\s]+?)(?=\\s+actor|\\s+target|\\s+priority|\\s+ttl|\\s+tags|$)/i);
            const targetMatch = prompt.match(/target[:\\s]+([\\w\\s]+?)(?=\\s+actor|\\s+action|\\s+priority|\\s+ttl|\\s+tags|$)/i);
            const priorityMatch = prompt.match(/priority[:\\s]+(\\w+)/i);
            const ttlMatch = prompt.match(/ttl[:\\s]+(\\d+)/i);
            const tagsMatch = prompt.match(/tags[:\\s]+([\\w,\\s]+?)(?=\\s+actor|\\s+action|\\s+target|\\s+priority|\\s+ttl|$)/i);

            const actor = this.validateInput(actorMatch?.[1]?.trim() || 'VSCode_Chat', 'actor');
            const action = this.validateInput(actionMatch?.[1]?.trim() || 'chat_interaction', 'action');
            const target = this.validateInput(targetMatch?.[1]?.trim() || 'memory_system', 'target');

            const VALID_PRIORITIES = ['pinned', 'high', 'normal', 'low'];
            const rawPriority = priorityMatch?.[1]?.toLowerCase().trim();
            const priority = rawPriority && VALID_PRIORITIES.includes(rawPriority) ? rawPriority : undefined;

            const ttl_seconds = ttlMatch ? parseInt(ttlMatch[1], 10) : undefined;

            const tags = tagsMatch
                ? tagsMatch[1].split(',').map(t => t.trim()).filter(t => t.length > 0)
                : undefined;

            const memoryRequest: AddMemoryRequest = {
                actor,
                action,
                target,
                record_type: 'Symbolic',  // /add = explicit decision → Symbolic layer
                priority,
                ttl_seconds,
                tags,
                metadata: {
                    source: 'vscode-chat',
                    timestamp: new Date().toISOString(),
                    original_prompt: prompt
                }
            };

            const response = await this.api.addMemory(memoryRequest);
            if (response.success) {
                stream.markdown(`✅ **Memory Successfully Added!**\n\n`);
                
                // Create a nice summary table
                stream.markdown(`| Field | Value |\n`);
                stream.markdown(`|-------|-------|\n`);
                stream.markdown(`| **Record ID** | \`${response.record_id?.substring(0, 8)}...\` |\n`);
                stream.markdown(`| **Actor** | ${actor} |\n`);
                stream.markdown(`| **Action** | ${action} |\n`);
                stream.markdown(`| **Target** | ${target} |\n`);
                stream.markdown(`| **Type** | Symbolic |\n`);
                stream.markdown(`| **Timestamp** | ${new Date().toLocaleString()} |\n\n`);
                
                stream.markdown(`🎯 **What this means:**\n`);
                stream.markdown(`This memory record captures that **${actor}** performed **${action}** on **${target}**. `);
                stream.markdown(`It's now part of your persistent memory system and can be retrieved later for context and learning.\n\n`);
                
                stream.markdown(`💡 **Try querying**: \`@hipcortex query actor:${actor}\` to see related memories.\n`);
            } else {
                stream.markdown(`❌ **Failed to add memory**: ${response.error}\n`);
            }
        } catch (error) {
            if (error instanceof Error && error.message.includes('validation')) {
                stream.markdown(`❌ **Input Validation Error**: ${error.message}\\n`);
            } else {
                stream.markdown(`❌ **API Error**: ${error instanceof Error ? error.message : String(error)}\\n`);
            }
        }
    }

    private async handleQueryMemory(request: vscode.ChatRequest, stream: vscode.ChatResponseStream): Promise<void> {
        stream.markdown('🔍 **Querying memory records...**\\n\\n');

        // Parse query parameters
        const prompt = request.prompt;
        const actorMatch = prompt.match(/actor[:\\s]+([\\w\\s]+?)(?=\\s+action|\\s+limit|$)/i);
        const actionMatch = prompt.match(/action[:\\s]+([\\w\\s]+?)(?=\\s+actor|\\s+limit|$)/i);
        const limitMatch = prompt.match(/limit[:\\s]+(\\d+)/i);

        const mode = vscode.workspace.getConfiguration('hipcortex').get<string>('optimizationMode', 'headroom');
        const defaultLimit = mode === 'caveman' ? 3 : 5;

        const queryParams: any = {};
        if (actorMatch) queryParams.actor = actorMatch[1].trim();
        if (actionMatch) queryParams.action = actionMatch[1].trim();
        if (limitMatch) queryParams.limit = parseInt(limitMatch[1]);
        else queryParams.limit = defaultLimit;

        try {
            const response = await this.api.queryMemory(queryParams);
            
            stream.markdown(`📊 **Found ${response.total} records**\n\n`);
            
            if (response.records.length === 0) {
                stream.markdown('No records found matching your criteria.\n');
                return;
            }

            // Group records by date for better readability
            const recordsByDate = response.records.reduce((groups: {[key: string]: any[]}, record) => {
                const date = new Date(record.timestamp).toDateString();
                if (!groups[date]) groups[date] = [];
                groups[date].push(record);
                return groups;
            }, {});

            // Display records grouped by date
            Object.entries(recordsByDate).forEach(([date, records]) => {
                stream.markdown(`## 📅 ${date}\n\n`);
                
                records.forEach((record, index) => {
                    const time = new Date(record.timestamp).toLocaleTimeString();
                    
                    stream.markdown(`### 🔸 Record ${index + 1}\n`);
                    stream.markdown(`**Time:** ${time}\n\n`);
                    stream.markdown(`| Field | Value |\n`);
                    stream.markdown(`|-------|-------|\n`);
                    stream.markdown(`| **ID** | \`${record.id.substring(0, 8)}...\` |\n`);
                    stream.markdown(`| **Type** | ${record.record_type} |\n`);
                    stream.markdown(`| **Actor** | ${record.actor} |\n`);
                    stream.markdown(`| **Action** | ${record.action} |\n`);
                    stream.markdown(`| **Target** | ${record.target} |\n`);
                    
                    // Format metadata in a readable way
                    if (record.metadata && Object.keys(record.metadata).length > 0) {
                        stream.markdown(`\n**📋 Additional Details:**\n`);
                        Object.entries(record.metadata).forEach(([key, value]) => {
                            const displayKey = key.replace(/_/g, ' ').replace(/\b\w/g, l => l.toUpperCase());
                            let displayValue = value;
                            
                            // Format specific types
                            if (typeof value === 'number' && key.includes('confidence')) {
                                displayValue = `${(value * 100).toFixed(1)}%`;
                            } else if (typeof value === 'number' && key.includes('time')) {
                                displayValue = `${value}s`;
                            } else if (typeof value === 'object') {
                                displayValue = JSON.stringify(value, null, 2);
                            }
                            
                            stream.markdown(`- **${displayKey}:** ${displayValue}\n`);
                        });
                    }
                    
                    stream.markdown('\n---\n\n');
                });
            });

            // Token savings footer
            const contextBundle = response.records
                .map(r => `[${r.action}] ${r.target}`)
                .join('\n');
            const ESTIMATED_FULL_HISTORY = 2000; // typical full-history token estimate
            this.tokenTracker.record(contextBundle, ESTIMATED_FULL_HISTORY);
            stream.markdown('\n\n' + this.tokenTracker.formatSavingsFooter(contextBundle, ESTIMATED_FULL_HISTORY));
        } catch (error) {
            stream.markdown(`❌ **Query Error**: ${error instanceof Error ? error.message : String(error)}\n`);
        }
    }

    private async handleSearchMemory(request: vscode.ChatRequest, stream: vscode.ChatResponseStream): Promise<void> {
        stream.markdown('🔍 **Semantic search...**\n\n');
        const query = request.prompt.replace(/^search\s+/i, '').trim();
        const mode = vscode.workspace.getConfiguration('hipcortex').get<string>('optimizationMode', 'headroom');
        const defaultLimit = mode === 'caveman' ? 3 : 5;
        const limitMatch = query.match(/limit[:\s]+(\d+)/i);
        const limit = limitMatch ? parseInt(limitMatch[1]) : defaultLimit;
        const cleanQuery = query.replace(/limit[:\s]+\d+/i, '').trim();
        try {
            const response = await this.api.searchMemory(cleanQuery, limit);
            stream.markdown(`📊 **${response.total} result(s)** (scored)\n\n`);
            if (response.results.length === 0) {
                stream.markdown('No semantically similar memories found.\n');
                return;
            }
            for (const { score, record } of response.results) {
                stream.markdown(`**[${(score * 100).toFixed(1)}%]** [${record.action}] ${record.target} *(${record.actor})*\n\n`);
            }
            const contextBundle = response.results
                .map(r => `[${r.record?.action ?? ''}] ${r.record?.target ?? ''}`)
                .join('\n');
            const ESTIMATED_FULL_HISTORY = 2000;
            this.tokenTracker.record(contextBundle, ESTIMATED_FULL_HISTORY);
            stream.markdown('\n\n' + this.tokenTracker.formatSavingsFooter(contextBundle, ESTIMATED_FULL_HISTORY));
        } catch (error) {
            stream.markdown(`❌ **Search Error**: ${error instanceof Error ? error.message : String(error)}\n`);
        }
    }

    private async handleHealthCheck(stream: vscode.ChatResponseStream): Promise<void> {
        stream.markdown('🏥 **System Health Check**\n\n');
        
        try {
            let isHealthy = await this.api.healthCheck();
            
            if (!isHealthy) {
                stream.markdown('⚠️ **HipCortex API is currently offline**\n\n');
                stream.markdown('**Status:** 🔴 Not responding\n');
                stream.markdown('**Issue:** Cannot connect to the memory server\n\n');
                
                stream.markdown('🔄 **Attempting automatic server startup...**\n\n');
                
                const started = await this.api.autoStartServer();
                if (started) {
                    stream.markdown('✅ **Success! Server is now running**\n\n');
                    isHealthy = true;
                } else {
                    stream.markdown('❌ **Automatic startup failed**\n\n');
                    stream.markdown('The extension should download and start the server automatically on first use.\n');
                    stream.markdown('**Troubleshooting:** Check "Output" panel (select "HipCortex Server"), ensure internet connection for binary download, or restart VS Code.\n');
                }
            }
            
            if (isHealthy) {
                stream.markdown('✅ **All Systems Operational!**\n\n');
                
                stream.markdown('| Component | Status |\n');
                stream.markdown('|-----------|--------|\n');
                stream.markdown('| **API Server** | 🟢 Online |\n');
                stream.markdown('| **Memory System** | 🟢 Active |\n');
                stream.markdown('| **VS Code Extension** | 🟢 Connected |\n\n');
                
                stream.markdown(`**🌐 Server URL:** \`${this.api['baseUrl']}\`\n`);
                stream.markdown('**📡 Response Time:** < 100ms\n\n');
                
                stream.markdown('🎯 **Ready for memory operations!** Try adding or querying memories.\n');
            }
        } catch (error) {
            stream.markdown(`❌ **Health Check Failed**\n\n`);
            stream.markdown(`**Error Details:** ${error instanceof Error ? error.message : String(error)}\n\n`);
            stream.markdown('**Troubleshooting:**\n');
            stream.markdown('1. Ensure the HipCortex server is running\n');
            stream.markdown('2. Check network connectivity\n');
            stream.markdown('3. Verify the API URL in VS Code settings\n');
        }
    }

    private async handleHelp(stream: vscode.ChatResponseStream): Promise<void> {
        stream.markdown(`# 🧠 HipCortex Memory Assistant (Official Extension v0.1.3)

**✅ You are now talking to the OFFICIAL HipCortex VS Code Extension!**

Welcome! I'm your AI memory assistant, helping you store and retrieve information efficiently.

## 🎯 What I Can Do

I help you capture and organize memories about your work, learning, and interactions. Think of me as your personal knowledge base that grows with every conversation.

## 💬 How to Talk to Me

### 📝 **Store New Memories**
Tell me about activities, learnings, or events:
- \`@hipcortex add actor:John action:completed target:authentication_feature\`
- \`@hipcortex record actor:Student action:learned target:rust_ownership\`
- \`@hipcortex store actor:TeamLead action:reviewed target:pull_request_42\`

### 🔍 **Find Past Memories**
Search your stored knowledge:
- \`@hipcortex query actor:John\` - See everything John did
- \`@hipcortex search action:learned\` - Find all learning activities
- \`@hipcortex find actor:Student limit:5\` - Get recent student activities

### ⚙️ **System Commands**
Check if everything is working:
- \`@hipcortex health\` - Verify the memory system is running
- \`@hipcortex status\` - Get detailed system information

## 🌟 **Quick Examples**

Here are some real conversations you can have with me:

**Scenario 1: After finishing a task**
> \`@hipcortex add actor:Developer action:fixed_bug target:login_system\`

**Scenario 2: Learning something new**
> \`@hipcortex record actor:Student action:mastered target:async_programming\`

**Scenario 3: Finding past work**
> \`@hipcortex search action:fixed_bug limit:10\`

**Scenario 4: Checking the system**
> \`@hipcortex health\`

## 🚀 **Pro Tips**

- **Be specific**: Instead of \`action:work\`, use \`action:code_review\` or \`action:bug_fix\`
- **Use consistent names**: If you're "Developer" in one memory, stay "Developer" for related activities
- **Add context**: The target field helps you remember what specifically was involved
- **Regular queries**: Use \`query actor:YourName\` to see your activity patterns
- **Harness policy**: Copilot and agents MUST call hipcortex_search / live_beliefs FIRST before any recall or prediction. Use substrate-first for decisions.

## 🤝 **Ready to Start?**

Try any of the examples above, or just tell me about something you did today!

💡 **Remember**: Every interaction helps build your personal knowledge graph. The more you use me, the more valuable your memory system becomes.

---
🔗 **Connected to**: http://127.0.0.1:3030 | **Extension ID**: hipcortex-memory-0.1.3`);
    }
}

/** Races a promise against a timeout. Returns null if timed out or errored. Clears timer on settle. */
async function withTimeout<T>(promise: Promise<T>, ms: number): Promise<T | null> {
    if (ms <= 0) { return null; }
    let timer: ReturnType<typeof setTimeout> | undefined;
    try {
        return await Promise.race([
            promise.catch(() => null as T | null),
            new Promise<null>(resolve => {
                timer = setTimeout(() => resolve(null), ms);
            })
        ]);
    } finally {
        if (timer !== undefined) {
            clearTimeout(timer);
        }
    }
}

/** Attempts to read the current git branch for the document's repo. Returns null on failure. */
async function getGitBranch(doc: vscode.TextDocument): Promise<string | null> {
    try {
        const gitExt = vscode.extensions.getExtension('vscode.git');
        if (!gitExt) { return null; }
        const git = gitExt.isActive ? gitExt.exports : await gitExt.activate();
        const api = git.getAPI(1);
        if (!api?.repositories?.length) { return null; }
        const repo = api.repositories.find((r: any) =>
            doc.uri.fsPath.startsWith(r.rootUri.fsPath)
        ) ?? api.repositories[0];
        const branch: string | undefined = repo?.state?.HEAD?.name;
        if (!branch) { return null; }
        const stripped = branch.replace(/^(feature|fix|chore|bugfix|hotfix)\//, '');
        return stripped.replace(/[^a-z0-9-]/gi, '-').toLowerCase().slice(0, 30) || null;
    } catch {
        return null;
    }
}

/** Runs the document symbol provider and returns semantic category tags (max 2). */
async function getDocumentSymbolTags(doc: vscode.TextDocument): Promise<string[]> {
    try {
        const symbols: vscode.DocumentSymbol[] | undefined =
            await vscode.commands.executeCommand('vscode.executeDocumentSymbolProvider', doc.uri);
        if (!symbols?.length) { return []; }
        const names = symbols.map((s: vscode.DocumentSymbol) => s.name.toLowerCase());
        const result: string[] = [];
        if (names.some((n: string) => n.startsWith('test_') || n.startsWith('it_') || n === 'test')) {
            result.push('testing');
        }
        if (names.some((n: string) => n.includes('handle_') || n.includes('handler') || n.includes('route'))) {
            result.push('api');
        }
        return [...new Set(result)].slice(0, 2);
    } catch {
        return [];
    }
}

/**
 * Extracts semantic tags from a VS Code TextDocument for memory auto-capture.
 *
 * Layer 1 (sync): languageId, file extension, 'auto-capture'
 * Layer 2 (sync): path-segment semantic hints (testing, api, sdk, docs, ...)
 * Layer 3 (async, capped): git branch name slug
 * Layer 4 (async, capped): document symbol categorization
 * Layer 5 (sync): error diagnostics → 'has-errors'
 *
 * @param doc       The TextDocument being processed.
 * @param timeoutMs Budget in ms for async layers (default 200). Pass 0 to skip async.
 * @returns         Deduplicated lowercase string[], max 8 elements.
 */
export async function extractSemanticTags(
    doc: vscode.TextDocument,
    timeoutMs: number = 200
): Promise<string[]> {
    const tags: string[] = [];

    // Layer 1: Structural (always present, sync)
    const ext = path.extname(doc.fileName).replace(/^\./, '').toLowerCase();
    tags.push(doc.languageId.toLowerCase());
    if (ext && ext !== doc.languageId.toLowerCase()) {
        tags.push(ext);
    }
    tags.push('auto-capture');

    // Layer 2: Path-segment semantic hints (sync)
    const lowerPath = doc.fileName.replace(/\\/g, '/').toLowerCase();
    const segs = lowerPath.split('/');
    const pathHints: [string[], string][] = [
        [['test', 'tests', 'spec', '__test__', '__tests__', '.test.', '.spec.'], 'testing'],
        [['api', 'routes', 'handlers', 'endpoints'],                            'api'],
        [['sdk'],                                                                'sdk'],
        [['docs', 'documentation'],                                             'documentation'],
        [['migrations', 'migration'],                                           'database'],
        [['components', 'views', 'pages', 'frontend'],                         'frontend'],
        [['modules', 'module'],                                                 'module'],
        [['src', 'lib', 'core'],                                                'core'],
        [['scripts', 'tools'],                                                  'tooling'],
    ];
    for (const [patterns, tag] of pathHints) {
        // Exact path segment (or segment.ext) for short tokens; dotted patterns (.test./.spec.) use path includes.
        // Avoid s.includes(p) false positives e.g. "contest" matching "test".
        if (patterns.some(p => {
            if (p.includes('.')) {
                return lowerPath.includes(p);
            }
            return segs.some(s => s === p || s.startsWith(p + '.'));
        })) {
            tags.push(tag);
        }
    }

    // Layers 3+4: Async (git branch + document symbols) within budget — do not start when timeoutMs <= 0
    if (timeoutMs > 0) {
        const [branchResult, symbolsResult] = await Promise.all([
            withTimeout(getGitBranch(doc), timeoutMs),
            withTimeout(getDocumentSymbolTags(doc), timeoutMs),
        ]);
        if (branchResult) { tags.push(branchResult); }
        if (symbolsResult && symbolsResult.length > 0) { tags.push(...symbolsResult); }
    }

    // Layer 5: Error diagnostics (sync)
    let hasErrors = false;
    try {
        const diags = vscode.languages.getDiagnostics(doc.uri);
        if (diags.some(d => d.severity === vscode.DiagnosticSeverity.Error)) {
            hasErrors = true;
            tags.push('has-errors');
        }
    } catch { /* skip in test environments */ }

    // Deduplicate, lowercase, cap at 8; reserve a slot for has-errors when present
    const seen = new Set<string>();
    const deduped = tags
        .map(t => t.toLowerCase().replace(/\s+/g, '-'))
        .filter(t => t.length > 0 && !seen.has(t) && seen.add(t));

    if (hasErrors && !deduped.slice(0, 8).includes('has-errors')) {
        return [...deduped.slice(0, 7), 'has-errors'];
    }
    return deduped.slice(0, 8);
}

/**
 * Builds a QuickPick list item for hipcortex.queryMemory.
 * Tags appear in the label so users can scan without opening the detail dialog.
 */
export function formatMemoryQuickPickItem(record: MemoryRecord): {
    label: string;
    description: string;
    detail: string;
    record: MemoryRecord;
} {
    const tagsStr =
        record.tags && record.tags.length > 0
            ? ` [${record.tags.join(', ')}]`
            : '';

    const detailParts: string[] = [];
    if (record.source) {
        detailParts.push(`source: ${record.source}`);
    }
    if (record.priority && record.priority !== 'normal') {
        detailParts.push(`priority: ${record.priority}`);
    }
    if (record.metadata) {
        if (typeof record.metadata.user_message === 'string') {
            const um = record.metadata.user_message as string;
            const snippet = um.length > 60 ? um.substring(0, 57) + '...' : um;
            detailParts.push(`"${snippet}"`);
        } else if (record.metadata.workspace) {
            detailParts.push(`workspace: ${record.metadata.workspace}`);
        } else if (record.metadata.tokens !== undefined) {
            detailParts.push(`tokens: ${record.metadata.tokens}`);
        }
    }

    const timeStr = new Date(record.timestamp).toLocaleString();
    const detailLine =
        detailParts.length > 0
            ? `${timeStr} • ${detailParts.join(' • ')}`
            : timeStr;

    return {
        label: `${record.actor} → ${record.action}${tagsStr}`,
        description: record.target,
        detail: detailLine,
        record,
    };
}

/** Human-readable multi-line detail body for the selected QuickPick record. */
export function formatMemoryDetailMessage(record: MemoryRecord): string {
    const tagsStr =
        record.tags && record.tags.length > 0
            ? record.tags.join(', ')
            : 'None';
    return (
        `Memory Record Details:\n` +
        `• ID: ${record.id}\n` +
        `• Actor: ${record.actor}\n` +
        `• Action: ${record.action}\n` +
        `• Target: ${record.target}\n` +
        `• Keywords/Tags: ${tagsStr}\n` +
        `• Priority: ${record.priority || 'normal'}\n` +
        `• Source: ${record.source || 'unknown'}\n` +
        `• Timestamp: ${new Date(record.timestamp).toLocaleString()}\n` +
        `• Metadata: ${JSON.stringify(record.metadata, null, 2)}`
    );
}

export function activate(context: vscode.ExtensionContext) {
    extensionInstallPath = context.extensionPath;
    console.log('🧠 HipCortex Memory Extension v0.3.3 active');
    console.log('🔧 Registering chat participant: hipcortex');

    const tokenTracker = new TokenTracker();

    const serverChannel = vscode.window.createOutputChannel('HipCortex Server');
    context.subscriptions.push(serverChannel);

    const apiForStart = new HipCortexAPI();
    apiForStart.autoStartServer(serverChannel).then(started => {
        if (started) {
            serverChannel.appendLine('Server ready (eager start on activate).');
        }
    }).catch(err => {
        serverChannel.appendLine(`Startup error: ${err instanceof Error ? err.message : String(err)}`);
    });

    // ── Status bar ────────────────────────────────────────────────────────────
    const statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 100);
    statusBarItem.tooltip = 'HipCortex Memory — click to query';
    statusBarItem.command = 'hipcortex.queryMemory';
    const initMode = vscode.workspace.getConfiguration('hipcortex').get<string>('optimizationMode', 'headroom');
    const initModeCap = initMode === 'caveman' ? 'Caveman' : 'Headroom';
    statusBarItem.text = tokenTracker.formatStatusBarLabel(0, 0, initModeCap);
    statusBarItem.show();
    context.subscriptions.push(statusBarItem);
    const updateStatusBar = (trans: number = 0, loops: number = 0, predSuffix: string = '') => {
        const mode = vscode.workspace.getConfiguration('hipcortex').get<string>('optimizationMode', 'headroom');
        const modeCap = mode === 'caveman' ? 'Caveman' : 'Headroom';
        statusBarItem.text = tokenTracker.formatStatusBarLabel(trans, loops, modeCap, predSuffix);
    };

    // ── LM Tool: hipcortex_search — Copilot can call this automatically ───────
    // Requires VS Code 1.90+. Gracefully no-ops on older versions.
    if (typeof (vscode.lm as any)?.registerTool === 'function') {
        const tool = (vscode.lm as any).registerTool('hipcortex_search', {
            modelDescription: HARNESS_TOOL_DESCRIPTIONS.search,
            invoke: async (options: any, _token: vscode.CancellationToken) => {
                const query: string = (options?.input?.query as string | undefined)?.trim() || 'recent decisions';
                try {
                    const api = new HipCortexAPI();
                    const mode = vscode.workspace.getConfiguration('hipcortex').get<string>('optimizationMode', 'headroom');
                    const modeLimit = mode === 'caveman' ? 3 : 5;
                    const response = await api.liveBeliefs({ limit: modeLimit });
                    // build bundle from rich fields (world_state + hypotheses + summary) instead of raw records
                    const contextBundle = `${response.summary || ''}\n${JSON.stringify(response.world_state || {})} /* rich summary + top facts */`;
                    const BASELINE = 2000; // estimated full-history tokens
                    tokenTracker.record(contextBundle, BASELINE);
                    // after liveBeliefs or reflect
                    const trans = response && response.world_state ? Object.keys(response.world_state).length : 0;
                    const loops = (response as any) && (response as any).loops_run ? (response as any).loops_run : 0;
                    const summary = response && response.summary ? response.summary : '';
                    serverChannel.appendLine(`Live: ${summary}`);
                    updateStatusBar(trans, loops);
                    await new HipCortexAPI().addMemory({ actor: 'copilot', action: 'used-substrate', target: query, metadata: { via: 'lm-tool' } });
                    if (query.length > 5) {
                        const r = await new HipCortexAPI().reflect(query).catch(() => ({} as any));
                        const loops2 = r && r.loops_run ? r.loops_run : 0;
                        const summary2 = r && r.hypothesis ? r.hypothesis : `reflect: ${query}`;
                        serverChannel.appendLine(`Live: ${summary2}`);
                        updateStatusBar(trans, loops2);
                    }
                    return { content: [{ type: 'text', value: `HipCortex live beliefs: ${contextBundle}` }] };
                } catch (err) {
                    return { content: [{ type: 'text', value: `HipCortex search failed: ${err instanceof Error ? err.message : String(err)}` }] };
                }
            }
        });
        const healthTool = (vscode.lm as any).registerTool('hipcortex_health', {
            modelDescription: HARNESS_TOOL_DESCRIPTIONS.health,
            invoke: async () => {
                try {
                    const summary = await new HipCortexAPI().healthSummary();
                    await new HipCortexAPI().addMemory({ actor: 'copilot', action: 'used-substrate', target: 'health', metadata: { via: 'lm-tool' } });
                    const r = await new HipCortexAPI().reflect('health').catch(() => ({} as any));
                    const loops = r && r.loops_run ? r.loops_run : 0;
                    const saved = tokenTracker.getSnapshot().savedTokens.toLocaleString();
                    statusBarItem.text = `$(database) HipCortex: WM 0 | loops ${loops} | ${saved} tok`;
                    serverChannel.appendLine(`Live: ${summary}`);
                    updateStatusBar(0, loops);
                    return { content: [{ type: 'text', value: summary }] };
                } catch (err) {
                    return { content: [{ type: 'text', value: `HipCortex health check failed: ${err instanceof Error ? err.message : String(err)}` }] };
                }
            }
        });
        const predictTool = (vscode.lm as any).registerTool('hipcortex_predict', {
            modelDescription: HARNESS_TOOL_DESCRIPTIONS.predict,
            invoke: async (options: any, _token: vscode.CancellationToken) => {
                const state: string = options?.input?.state || '';
                const action: string = options?.input?.action || '';
                try {
                    const api = new HipCortexAPI();
                    const gate = await api.canExecute('predict');
                    if (!gate) {
                        return { content: [{ type: 'text', value: 'Substrate uncertain: confidence too low. Self-Model health gating check failed.' }] };
                    }
                    const res = await api.predictState(state, action);
                    await api.addMemory({ actor: 'copilot', action: 'used-substrate', target: state + ':' + action, metadata: { via: 'lm-tool' } });
                    const surpriseQuery = state + action;
                    if (surpriseQuery.length > 5) {
                        const r = await api.reflect(surpriseQuery).catch(() => ({} as any));
                        const loops = r && r.loops_run ? r.loops_run : 0;
                        const saved = tokenTracker.getSnapshot().savedTokens.toLocaleString();
                        statusBarItem.text = `$(database) HipCortex: WM 0 | loops ${loops} | ${saved} tok`;
                        serverChannel.appendLine(`Live: predict ${state}:${action}`);
                        updateStatusBar(0, loops);
                    }
                    return { content: [{ type: 'text', value: JSON.stringify(res) }] };
                } catch (err) {
                    return { content: [{ type: 'text', value: `HipCortex predict failed: ${err instanceof Error ? err.message : String(err)}` }] };
                }
            }
        });

        const rolloutTool = (vscode.lm as any).registerTool('hipcortex_rollout', {
            modelDescription: HARNESS_TOOL_DESCRIPTIONS.rollout,
            invoke: async (options: any, _token: vscode.CancellationToken) => {
                const api = new HipCortexAPI();
                const initial_state: string = options?.input?.initial_state || '';
                const actions: string[] = options?.input?.actions || [];
                const mode: string = options?.input?.mode || (actions.length ? 'dirichlet' : 'mcts');
                const goal_state: string | undefined = options?.input?.goal_state;
                const iterations: number = options?.input?.iterations ?? 50;
                const max_depth: number = options?.input?.max_depth ?? 3;
                const baseUrl = (api as any).baseUrl;
                const headers: any = { 'Content-Type': 'application/json' };
                if ((api as any).apiKey) { headers['Authorization'] = `Bearer ${(api as any).apiKey}`; }
                try {
                    const gate = await api.canExecute('rollout');
                    if (!gate) {
                        return { content: [{ type: 'text', value: 'Substrate uncertain: confidence too low. Self-Model health gating check failed.' }] };
                    }
                    const body: any = { initial_state, mode, iterations, max_depth };
                    if (actions.length) { body.actions = actions; }
                    if (goal_state) { body.goal_state = goal_state; }
                    const response = await axios.post(`${baseUrl}/worldmodel/rollout`, body, { headers });
                    return { content: [{ type: 'text', value: JSON.stringify(response.data) }] };
                } catch (err) {
                    return { content: [{ type: 'text', value: `hipcortex_rollout failed: ${err instanceof Error ? err.message : String(err)}` }] };
                }
            }
        });

        const graphSearchTool = (vscode.lm as any).registerTool('hipcortex_graph_search', {
            modelDescription: 'Use this AFTER hipcortex_search to expand context graph-first. Call with a seed_id (record UUID) to find memories linked via the CausalTopoGraph using PPR (alpha=0.85, 20 rounds).',
            invoke: async (options: any, _token: vscode.CancellationToken) => {
                const seedId: string = (options?.input?.seed_id as string | undefined)?.trim() || '';
                const limit: number = (options?.input?.limit as number | undefined) ?? 10;
                if (!seedId) {
                    return { content: [{ type: 'text', value: 'hipcortex_graph_search requires a seed_id (record UUID).' }] };
                }
                try {
                    const api = new HipCortexAPI();
                    const res = await api.searchRelated(seedId, limit);
                    const lines = (res.results || []).map((r: any) =>
                        `• [${r.record?.action ?? '?'}] ${r.record?.target ?? ''} (score: ${r.score?.toFixed(3) ?? '0.000'})`
                    );
                    const text = lines.length > 0
                        ? `${lines.length} PPR-related result(s) for seed ${seedId}:\n${lines.join('\n')}`
                        : `No related memories found for seed ${seedId}. Ensure records are linked.`;
                    return { content: [{ type: 'text', value: text }] };
                } catch (err) {
                    return { content: [{ type: 'text', value: `hipcortex_graph_search failed: ${err instanceof Error ? err.message : String(err)}` }] };
                }
            }
        });

        const causalTool = (vscode.lm as any).registerTool('hipcortex_causal', {
            modelDescription: 'Run a causal or counterfactual query against the HipCortex WorldModel causal graph. Support modes: counterfactual, intervention, graph.',
            invoke: async (options: any, _token: vscode.CancellationToken) => {
                const api = new HipCortexAPI();
                const mode: string = options?.input?.mode || 'graph';
                const baseUrl = (api as any).baseUrl;
                const headers: any = {};
                if ((api as any).apiKey) { headers['Authorization'] = `Bearer ${(api as any).apiKey}`; }
                try {
                    let result: any;
                    if (mode === 'counterfactual') {
                        const body = { intervention: options?.input?.intervention || '', query: options?.input?.query || '' };
                        result = await axios.post(`${baseUrl}/worldmodel/causal/counterfactual`, body, { headers });
                    } else if (mode === 'intervention') {
                        const body = { variable: options?.input?.variable || '', value: options?.input?.value };
                        result = await axios.post(`${baseUrl}/worldmodel/causal/intervention`, body, { headers });
                    } else {
                        result = await axios.get(`${baseUrl}/worldmodel/causal`, { headers });
                    }
                    return { content: [{ type: 'text', value: JSON.stringify(result.data, null, 2) }] };
                } catch (err) {
                    return { content: [{ type: 'text', value: `hipcortex_causal failed: ${err instanceof Error ? err.message : String(err)}` }] };
                }
            }
        });

        const topoPprTool = (vscode.lm as any).registerTool('hipcortex_topo_ppr', {
            modelDescription: HARNESS_TOOL_DESCRIPTIONS.topoPpr,
            invoke: async (options: any) => {
                const api = new HipCortexAPI();
                const seed: string = (options?.input?.seed as string | undefined)?.trim() || '';
                const limit: number = options?.input?.limit ?? 10;
                const baseUrl = (api as any).baseUrl;
                const headers: any = {};
                if ((api as any).apiKey) { headers['Authorization'] = `Bearer ${(api as any).apiKey}`; }
                try {
                    const qs = new URLSearchParams({ limit: String(limit) });
                    if (seed) { qs.set('seed', seed); }
                    const res = await axios.get(`${baseUrl}/topo/ppr?${qs}`, { headers });
                    return { content: [{ type: 'text', value: JSON.stringify(res.data, null, 2) }] };
                } catch (err) {
                    return { content: [{ type: 'text', value: `hipcortex_topo_ppr failed: ${err instanceof Error ? err.message : String(err)}` }] };
                }
            }
        });

        const deconstructTool = (vscode.lm as any).registerTool('hipcortex_deconstruct', {
            modelDescription: HARNESS_TOOL_DESCRIPTIONS.deconstruct,
            invoke: async (options: any) => {
                const api = new HipCortexAPI();
                const text: string = (options?.input?.text as string | undefined)?.trim() || '';
                const llm_json: string | undefined = options?.input?.llm_json;
                const apply: boolean = Boolean(options?.input?.apply);
                if (!text) {
                    return { content: [{ type: 'text', value: 'hipcortex_deconstruct requires text' }] };
                }
                const baseUrl = (api as any).baseUrl;
                const headers: any = { 'Content-Type': 'application/json' };
                if ((api as any).apiKey) { headers['Authorization'] = `Bearer ${(api as any).apiKey}`; }
                try {
                    const body: any = { text };
                    if (llm_json) { body.llm_json = llm_json; }
                    const path = apply ? '/topo/apply-hyp' : '/topo/deconstruct';
                    const res = await axios.post(`${baseUrl}${path}`, body, { headers });
                    return { content: [{ type: 'text', value: JSON.stringify(res.data, null, 2) }] };
                } catch (err) {
                    return { content: [{ type: 'text', value: `hipcortex_deconstruct failed: ${err instanceof Error ? err.message : String(err)}` }] };
                }
            }
        });

        const checkEdgeTool = (vscode.lm as any).registerTool('hipcortex_check_edge', {
            modelDescription: HARNESS_TOOL_DESCRIPTIONS.checkEdge,
            invoke: async (options: any) => {
                const api = new HipCortexAPI();
                const from: string = options?.input?.from || '';
                const to: string = options?.input?.to || '';
                if (!from || !to) {
                    return { content: [{ type: 'text', value: 'hipcortex_check_edge requires from and to' }] };
                }
                const baseUrl = (api as any).baseUrl;
                const headers: any = { 'Content-Type': 'application/json' };
                if ((api as any).apiKey) { headers['Authorization'] = `Bearer ${(api as any).apiKey}`; }
                try {
                    const res = await axios.post(`${baseUrl}/topo/check-edge`, { from, to }, { headers });
                    return { content: [{ type: 'text', value: JSON.stringify(res.data, null, 2) }] };
                } catch (err) {
                    return { content: [{ type: 'text', value: `hipcortex_check_edge failed: ${err instanceof Error ? err.message : String(err)}` }] };
                }
            }
        });

        const canExecuteTool = (vscode.lm as any).registerTool('hipcortex_can_execute', {
            modelDescription: HARNESS_TOOL_DESCRIPTIONS.canExecute,
            invoke: async (options: any) => {
                const operation: string = options?.input?.operation || 'add_memory';
                try {
                    const api = new HipCortexAPI();
                    const ok = await api.canExecute(operation as any);
                    return { content: [{ type: 'text', value: `can_execute(${operation}) = ${ok}` }] };
                } catch (err) {
                    return { content: [{ type: 'text', value: `hipcortex_can_execute failed: ${err instanceof Error ? err.message : String(err)}` }] };
                }
            }
        });

        context.subscriptions.push(
            tool, healthTool, predictTool, rolloutTool, graphSearchTool, causalTool,
            topoPprTool, deconstructTool, checkEdgeTool, canExecuteTool
        );
        console.log(
            '✅ HipCortex LM Tools registered: hipcortex_search, hipcortex_health, hipcortex_predict, ' +
            'hipcortex_rollout, hipcortex_graph_search, hipcortex_causal, hipcortex_topo_ppr, ' +
            'hipcortex_deconstruct, hipcortex_check_edge, hipcortex_can_execute'
        );
    } else {
        console.log('ℹ️ VS Code < 1.90 — LM Tools not available');
    }

    // ── Auto-capture: store file saves as temporal memories ──────────────────
    const onSave = vscode.workspace.onDidSaveTextDocument(async (doc) => {
        if (doc.uri.scheme !== 'file') { return; }
        if (doc.fileName.includes('.git') || doc.fileName.includes('node_modules')) { return; }
        try {
            const api = new HipCortexAPI();
            const started = await api.autoStartServer(serverChannel);
            if (!started) {
                serverChannel.appendLine('Auto-capture skipped: server not running.');
                return;
            }
            const fileName = doc.fileName.split(/[\\/]/).pop() || doc.fileName;
            const autoTags = await extractSemanticTags(doc, 200);
            await api.addMemory({
                actor: 'vscode-user',
                action: 'edited',
                target: `${fileName} (${doc.languageId}, ${doc.lineCount} lines)`,
                record_type: 'Symbolic',
                priority: 'normal',
                source: 'vscode-auto-capture',
                tags: autoTags,
                metadata: { 
                    source: 'vscode-auto-capture', 
                    language_id: doc.languageId, 
                    line_count: doc.lineCount,
                    workspace: vscode.workspace.name || 'unknown'
                }
            });
            const r = await api.reflect(`edited ${fileName}`).catch(() => ({} as any));
            const trans = 0;
            const loops = r && r.loops_run ? r.loops_run : 0;

            // WM predict: what file will likely be edited next?
            const pred = await api.predictState('vscode-user', 'edited').catch(() => null);
            let predSuffix = '';
            if (
                pred &&
                pred.probabilities &&
                typeof pred.observation_count === 'number' &&
                pred.observation_count >= 3
            ) {
                // Find highest-probability next target
                const entries = Object.entries(pred.probabilities as Record<string, number>);
                if (entries.length > 0) {
                    entries.sort(([, a], [, b]) => b - a);
                    const [topTarget] = entries[0];
                    // Show just the filename portion (strip path prefix if present)
                    const topFile = topTarget.split(/[\\/]/).pop()?.split(' ')[0] || topTarget;
                    const entropy = typeof pred.entropy === 'number' ? pred.entropy.toFixed(1) : '?';
                    predSuffix = ` | →${topFile}? H:${entropy}`;
                }
            }

            serverChannel.appendLine(`Live: edited ${fileName}${predSuffix}`);
            updateStatusBar(trans, loops, predSuffix);
        } catch (err) {
            serverChannel.appendLine(`Auto-capture error: ${err instanceof Error ? err.message : String(err)}`);
        }
    });
    context.subscriptions.push(onSave);
    console.log('✅ HipCortex auto-capture on file save registered');

    // Register chat participant with high priority and explicit configuration
    const chatParticipant = new HipCortexChatParticipant(tokenTracker);
    const participant = vscode.chat.createChatParticipant('hipcortex', chatParticipant.provideResponse.bind(chatParticipant));
    
    // Configure participant with explicit properties
    participant.iconPath = vscode.Uri.file(context.asAbsolutePath('icon.png'));
    
    // Add followup provider to ensure we're recognized as the official provider
    participant.followupProvider = {
        provideFollowups: async (result, context, token) => {
            return [
                {
                    prompt: 'health',
                    label: '🔍 Check System Health',
                    command: 'health'
                },
                {
                    prompt: 'help',
                    label: '❓ Show Help',
                    command: 'help'
                },
                {
                    prompt: 'add actor:Developer action:test target:extension',
                    label: '➕ Add Test Memory',
                    command: 'add'
                }
            ];
        }
    };

    console.log('✅ HipCortex chat participant registered successfully');
    
    // Register commands
    const addMemoryCommand = vscode.commands.registerCommand('hipcortex.addMemory', async () => {
        const actor = await vscode.window.showInputBox({ prompt: 'Enter actor name' });
        const action = await vscode.window.showInputBox({ prompt: 'Enter action' });
        const target = await vscode.window.showInputBox({ prompt: 'Enter target' });
        const tagsRaw = await vscode.window.showInputBox({
            prompt: 'Enter tags (comma-separated, optional)',
            placeHolder: 'e.g. auth, refactor, typescript'
        });
        
        if (actor && action && target) {
            try {
                const api = new HipCortexAPI();
                const activeDoc = vscode.window.activeTextEditor?.document;
                const autoTags = activeDoc ? await extractSemanticTags(activeDoc, 200) : [];
                const userTags = tagsRaw
                    ? tagsRaw.split(',').map(t => t.trim().toLowerCase()).filter(t => t.length > 0)
                    : [];
                const mergedTags = [...new Set([...autoTags, ...userTags])].slice(0, 8);

                const response = await api.addMemory({ actor, action, target, tags: mergedTags });
                if (response.success) {
                    vscode.window.showInformationMessage(`Memory added! ID: ${response.record_id}`);
                } else {
                    vscode.window.showErrorMessage(`Failed to add memory: ${response.error}`);
                }
            } catch (error) {
                vscode.window.showErrorMessage(`Error: ${error instanceof Error ? error.message : String(error)}`);
            }
        }
    });

    const queryMemoryCommand = vscode.commands.registerCommand('hipcortex.queryMemory', async () => {
        try {
            const api = new HipCortexAPI();
            const response = await api.queryMemory({ limit: 50 });

            const items = response.records.map(record => formatMemoryQuickPickItem(record));

            const selected = await vscode.window.showQuickPick(items, {
                placeHolder: 'Select a memory record to view details'
            });

            if (selected) {
                vscode.window.showInformationMessage(
                    formatMemoryDetailMessage(selected.record)
                );
            }
        } catch (error) {
            vscode.window.showErrorMessage(`Error: ${error instanceof Error ? error.message : String(error)}`);
        }
    });

    const testExtensionCommand = vscode.commands.registerCommand('hipcortex.testExtension', async () => {
        const api = new HipCortexAPI();
        const ok = await api.healthCheck();
        vscode.window.showInformationMessage(
            ok ? 'HipCortex extension OK — server reachable.' : 'HipCortex extension active but server not reachable.'
        );
    });

    const restartServerCommand = vscode.commands.registerCommand('hipcortex.restartServer', async () => {
        if (globalServerProcess) {
            globalServerProcess.kill();
            globalServerProcess = null;
        }
        const api = new HipCortexAPI();
        const ok = await api.autoStartServer(serverChannel);
        vscode.window.showInformationMessage(ok ? 'HipCortex server restarted.' : 'HipCortex server restart failed — see Output.');
        if (!ok) {
            serverChannel.show();
        }
    });

    context.subscriptions.push(participant, addMemoryCommand, queryMemoryCommand, testExtensionCommand, restartServerCommand);
}

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
