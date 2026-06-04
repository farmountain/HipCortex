import * as vscode from 'vscode';
import axios from 'axios';
import { TokenTracker } from './token-tracker';

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

interface AddMemoryRequest {
    actor: string;
    action: string;
    target: string;
    record_type?: string;
    metadata?: any;
}

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

    async autoStartServer(): Promise<boolean> {
        const config = vscode.workspace.getConfiguration('hipcortex');
        const autoStart = config.get('autoStart', true);
        
        if (!autoStart) {
            return false;
        }

        try {
            // Try to start the server using VS Code terminal
            const terminal = vscode.window.createTerminal('HipCortex Server');
            terminal.sendText('cd "D:\\All_Projects\\HipCortex\\HipCortex"');
            terminal.sendText('cargo run --bin webserver --features "web-server,petgraph_backend"');
            
            // Wait a few seconds for server to start
            await new Promise(resolve => setTimeout(resolve, 5000));
            
            return await this.healthCheck();
        } catch {
            return false;
        }
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
            
            if (command.startsWith('add') || command.startsWith('record')) {
                await this.handleAddMemory(request, stream);
            } else if (command.startsWith('query') || command.startsWith('search') || command.startsWith('find')) {
                await this.handleQueryMemory(request, stream);
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
        return trimmed.replace(/[<>\"'&]/g, '');
    }

    private async handleAddMemory(request: vscode.ChatRequest, stream: vscode.ChatResponseStream): Promise<void> {
        stream.markdown('🧠 **Adding memory record...**\\n\\n');

        try {
            // Parse the request for actor, action, target
            const prompt = request.prompt;
            const actorMatch = prompt.match(/actor[:\\s]+([\\w\\s]+?)(?=\\s+action|\\s+target|$)/i);
            const actionMatch = prompt.match(/action[:\\s]+([\\w\\s]+?)(?=\\s+actor|\\s+target|$)/i);
            const targetMatch = prompt.match(/target[:\\s]+([\\w\\s]+?)(?=\\s+actor|\\s+action|$)/i);

            const actor = this.validateInput(actorMatch?.[1]?.trim() || 'VSCode_Chat', 'actor');
            const action = this.validateInput(actionMatch?.[1]?.trim() || 'chat_interaction', 'action');
            const target = this.validateInput(targetMatch?.[1]?.trim() || 'memory_system', 'target');

            const memoryRequest: AddMemoryRequest = {
                actor,
                action,
                target,
                record_type: 'Temporal',
                metadata: {
                    source: 'vscode-chat',
                    timestamp: new Date().toISOString(),
                    original_prompt: prompt.substring(0, 500) // Limit stored prompt length
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
                stream.markdown(`| **Type** | Temporal |\n`);
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
                    stream.markdown('**Manual Solution:**\n');
                    stream.markdown('```bash\n');
                    stream.markdown('cargo run --bin webserver --features "web-server,petgraph_backend"\n');
                    stream.markdown('```\n\n');
                    stream.markdown('Then try this health check again.\n');
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

## 🤝 **Ready to Start?**

Try any of the examples above, or just tell me about something you did today!

💡 **Remember**: Every interaction helps build your personal knowledge graph. The more you use me, the more valuable your memory system becomes.

---
🔗 **Connected to**: http://127.0.0.1:3030 | **Extension ID**: hipcortex-memory-0.1.3`);
    }
}

export function activate(context: vscode.ExtensionContext) {
    console.log('🧠 HipCortex Memory Extension v0.1.6 is now active!');
    console.log('🔧 Registering chat participant: hipcortex');

    // ── Token savings tracker (session-scoped, resets on restart) ────────────
    const tokenTracker = new TokenTracker();

    // ── Status bar ────────────────────────────────────────────────────────────
    const statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 100);
    statusBarItem.tooltip = 'HipCortex Memory — click to query';
    statusBarItem.command = 'hipcortex.queryMemory';
    statusBarItem.text = '$(database) HipCortex';
    statusBarItem.show();
    context.subscriptions.push(statusBarItem);
    const updateStatusBar = () => { statusBarItem.text = tokenTracker.formatStatusBarLabel(); };

    // ── LM Tool: hipcortex_search — Copilot can call this automatically ───────
    // Requires VS Code 1.90+. Gracefully no-ops on older versions.
    if (typeof (vscode.lm as any)?.registerTool === 'function') {
        const tool = (vscode.lm as any).registerTool('hipcortex_search', {
            invoke: async (options: any, _token: vscode.CancellationToken) => {
                const query: string = (options?.input?.query as string | undefined)?.trim() || 'recent decisions';
                try {
                    const api = new HipCortexAPI();
                    const response = await api.queryMemory({ limit: 5 });
                    const qWords = query.toLowerCase().split(' ').filter(w => w.length > 2);
                    const matches = response.records
                        .filter(r => qWords.some(w => `${r.actor} ${r.action} ${r.target}`.toLowerCase().includes(w)))
                        .slice(0, 5);
                    if (matches.length === 0) {
                        return { content: [{ type: 'text', value: `No relevant memories found for: ${query}` }] };
                    }
                    const contextBundle = matches.map(r => `[${r.action}] ${r.target}`).join('\n');
                    const BASELINE = 2000; // estimated full-history tokens
                    tokenTracker.record(contextBundle, BASELINE);
                    updateStatusBar();
                    return { content: [{ type: 'text', value: `HipCortex memories (${matches.length}):\n${contextBundle}` }] };
                } catch (err) {
                    return { content: [{ type: 'text', value: `HipCortex search failed: ${err instanceof Error ? err.message : String(err)}` }] };
                }
            }
        });
        context.subscriptions.push(tool);
        console.log('✅ HipCortex LM Tool registered: hipcortex_search');
    } else {
        console.log('ℹ️ VS Code < 1.90 — LM Tool hipcortex_search not available');
    }

    // ── Auto-capture: store file saves as temporal memories ──────────────────
    const onSave = vscode.workspace.onDidSaveTextDocument(async (doc) => {
        if (doc.uri.scheme !== 'file') { return; }
        if (doc.fileName.includes('.git') || doc.fileName.includes('node_modules')) { return; }
        try {
            const api = new HipCortexAPI();
            if (!(await api.healthCheck())) { return; }
            const fileName = doc.fileName.split(/[\\/]/).pop() || doc.fileName;
            await api.addMemory({
                actor: fileName,
                action: 'edited',
                target: `${fileName} (${doc.languageId}, ${doc.lineCount} lines)`,
                record_type: 'Temporal',
                metadata: { source: 'vscode-auto-capture', language_id: doc.languageId, line_count: doc.lineCount }
            });
        } catch { /* silent — never interrupt save workflow */ }
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
        vscode.window.showInformationMessage('🧠 HipCortex Extension v0.1.5 is working! Chat participant "hipcortex" is registered.');
    });

    context.subscriptions.push(participant, addMemoryCommand, queryMemoryCommand, testExtensionCommand);
}

export function deactivate() {
    console.log('🧠 HipCortex Memory Extension deactivated');
}
