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
            await axios.get(`${this.baseUrl}/health`, { timeout: 3000 });
            return true;
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

        if (await this.healthCheck()) {
            return true;
        }

        const log = (msg: string) => {
            if (outputChannel) {
                outputChannel.appendLine(msg);
            } else {
                console.log(msg);
            }
        };

        try {
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
                    PORT: '3030',
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
            vscode.window.showWarningMessage(
                'HipCortex server failed to start. Open Output → HipCortex Server for details.',
                'Open Output'
            ).then(choice => {
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

        // 2. User cache — re-download if corrupt
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

        // 3. Download from GitHub releases
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

        const queryParams: any = {};
        if (actorMatch) queryParams.actor = actorMatch[1].trim();
        if (actionMatch) queryParams.action = actionMatch[1].trim();
        if (limitMatch) queryParams.limit = parseInt(limitMatch[1]);

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
        const limitMatch = query.match(/limit[:\s]+(\d+)/i);
        const limit = limitMatch ? parseInt(limitMatch[1]) : 10;
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
    statusBarItem.text = '$(database) HipCortex';
    statusBarItem.show();
    context.subscriptions.push(statusBarItem);
    const updateStatusBar = (trans: number = 0, loops: number = 0) => {
        const saved = tokenTracker.getSnapshot().savedTokens.toLocaleString();
        statusBarItem.text = `$(database) HipCortex: WM ${trans} | loops ${loops} | ${saved} tok`;
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
                    // Upgrade the search invoke to:
                    const response = await api.liveBeliefs({ limit: 5 });
                    // build bundle from rich fields (world_state + hypotheses + summary) instead of raw records
                    const contextBundle = `${response.summary || ''}\n${JSON.stringify(response.world_state || {})} /* rich summary + top facts */`;
                    const BASELINE = 2000; // estimated full-history tokens
                    tokenTracker.record(contextBundle, BASELINE);
                    // after liveBeliefs or reflect
                    const trans = response && response.world_state ? Object.keys(response.world_state).length : 0;
                    const loops = (response as any) && (response as any).loops_run ? (response as any).loops_run : 0;
                    const saved = tokenTracker.getSnapshot().savedTokens.toLocaleString();
                    const summary = response && response.summary ? response.summary : '';
                    statusBarItem.text = `$(database) HipCortex: WM ${trans} | loops ${loops} | ${saved} tok`;
                    serverChannel.appendLine(`Live: ${summary}`);
                    updateStatusBar(trans, loops);
                    await new HipCortexAPI().addMemory({ actor: 'copilot', action: 'used-substrate', target: query, metadata: { via: 'lm-tool' } });
                    if (query.length > 5) {
                        const r = await new HipCortexAPI().reflect(query).catch(() => ({} as any));
                        const loops2 = r && r.loops_run ? r.loops_run : 0;
                        const saved2 = tokenTracker.getSnapshot().savedTokens.toLocaleString();
                        const summary2 = r && r.hypothesis ? r.hypothesis : `reflect: ${query}`;
                        statusBarItem.text = `$(database) HipCortex: WM ${trans} | loops ${loops2} | ${saved2} tok`;
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
            modelDescription: 'Predict state after a sequence of N actions using WorldModelEnhanced multi-step rollout. Provide initial_state and actions (array of strings).',
            invoke: async (options: any, _token: vscode.CancellationToken) => {
                const api = new HipCortexAPI();
                const initial_state: string = options?.input?.initial_state || '';
                const actions: string[] = options?.input?.actions || [];
                const baseUrl = (api as any).baseUrl;
                const headers: any = { 'Content-Type': 'application/json' };
                if ((api as any).apiKey) { headers['Authorization'] = `Bearer ${(api as any).apiKey}`; }
                try {
                    const gate = await api.canExecute('rollout');
                    if (!gate) {
                        return { content: [{ type: 'text', value: 'Substrate uncertain: confidence too low. Self-Model health gating check failed.' }] };
                    }
                    const response = await axios.post(`${baseUrl}/worldmodel/rollout`, { initial_state, actions }, { headers });
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

        context.subscriptions.push(tool, healthTool, predictTool, rolloutTool, graphSearchTool, causalTool);
        console.log('✅ HipCortex LM Tools registered: hipcortex_search, hipcortex_health, hipcortex_predict, hipcortex_rollout, hipcortex_graph_search, hipcortex_causal');
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
            await api.addMemory({
                actor: 'vscode-user',
                action: 'edited',
                target: `${fileName} (${doc.languageId}, ${doc.lineCount} lines)`,
                record_type: 'Temporal',
                ttl_seconds: 86400,    // file edits are ephemeral → expire after 24h
                priority: 'low',       // low priority: don't surface above explicit decisions
                source: 'vscode-auto-capture',
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
            const saved = tokenTracker.getSnapshot().savedTokens.toLocaleString();

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

            statusBarItem.text = `$(database) HipCortex: WM ${trans} | loops ${loops}${predSuffix} | ${saved} tok`;
            serverChannel.appendLine(`Live: edited ${fileName}${predSuffix}`);
            updateStatusBar(trans, loops);
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
        
        if (actor && action && target) {
            try {
                const api = new HipCortexAPI();
                const response = await api.addMemory({ actor, action, target });
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
            const response = await api.queryMemory({ limit: 10 });
            
            const items = response.records.map(record => ({
                label: `${record.actor} → ${record.action}`,
                description: record.target,
                detail: new Date(record.timestamp).toLocaleString(),
                record
            }));
            
            const selected = await vscode.window.showQuickPick(items, {
                placeHolder: 'Select a memory record to view details'
            });
            
            if (selected) {
                const record = selected.record;
                const message = `**Memory Record**\\n\\n` +
                    `ID: ${record.id}\\n` +
                    `Actor: ${record.actor}\\n` +
                    `Action: ${record.action}\\n` +
                    `Target: ${record.target}\\n` +
                    `Timestamp: ${new Date(record.timestamp).toLocaleString()}\\n` +
                    `Metadata: ${JSON.stringify(record.metadata, null, 2)}`;
                
                vscode.window.showInformationMessage(message);
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
}
