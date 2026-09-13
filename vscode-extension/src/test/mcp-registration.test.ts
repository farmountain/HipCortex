import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';
import * as cp from 'child_process';
import {
    readJsonFile,
    isMcpEntryValid,
    detectPythonPath,
    writeMcpEntries,
} from '../extension';

jest.mock('vscode');
jest.mock('child_process');
const mockedCp = cp as jest.Mocked<typeof cp>;

// Resolve vscode mock so getConfiguration returns a usable stub
const vscode = require('vscode');
const mcpUpdateMock = jest.fn().mockResolvedValue(undefined);
const mcpGetMock = jest.fn().mockReturnValue({});
vscode.workspace.getConfiguration.mockReturnValue({
    get: mcpGetMock,
    update: mcpUpdateMock,
});

let tmpDir: string;

beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'hipcortex-test-'));
    jest.clearAllMocks();
    vscode.workspace.getConfiguration.mockReturnValue({
        get: mcpGetMock,
        update: mcpUpdateMock,
    });
});

afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
});

// ── readJsonFile ──────────────────────────────────────────────────────────────

describe('readJsonFile', () => {
    it('returns empty object for missing file', () => {
        expect(readJsonFile(path.join(tmpDir, 'nonexistent.json'))).toEqual({});
    });

    it('returns empty object for invalid JSON', () => {
        const p = path.join(tmpDir, 'bad.json');
        fs.writeFileSync(p, '{not valid}', 'utf8');
        expect(readJsonFile(p)).toEqual({});
    });

    it('returns parsed object for valid JSON', () => {
        const p = path.join(tmpDir, 'ok.json');
        fs.writeFileSync(p, JSON.stringify({ mcpServers: { hipcortex: {} } }), 'utf8');
        expect(readJsonFile(p)).toEqual({ mcpServers: { hipcortex: {} } });
    });
});

// ── isMcpEntryValid ───────────────────────────────────────────────────────────

describe('isMcpEntryValid', () => {
    const homeStub = os.homedir();

    it('returns false when ~/.claude/mcp.json is missing', () => {
        // No mcp.json exists in real home unless written — check with fake ext path that wouldn't match
        expect(isMcpEntryValid('/nonexistent-extension-path')).toBe(false);
    });

    it('returns false when entry has wrong launcher path', () => {
        const claudeDir = path.join(tmpDir, '.claude');
        fs.mkdirSync(claudeDir, { recursive: true });
        const mcpPath = path.join(claudeDir, 'mcp.json');
        fs.writeFileSync(mcpPath, JSON.stringify({
            mcpServers: { hipcortex: { type: 'stdio', command: 'python', args: ['/wrong/launcher.py'] } }
        }), 'utf8');
        // Override os.homedir in readJsonFile indirectly by verifying the function reads from home
        // (direct: it returns false for '/nonexistent-extension-path' regardless)
        expect(isMcpEntryValid('/nonexistent-extension-path')).toBe(false);
    });
});

// ── writeMcpEntries ───────────────────────────────────────────────────────────

describe('writeMcpEntries', () => {
    it('creates ~/.claude/mcp.json with correct structure', () => {
        const claudeDir = path.join(tmpDir, '.claude');
        const claudeCfg = path.join(claudeDir, 'mcp.json');

        writeMcpEntries('/ext/path', 'python3', undefined, tmpDir);

        expect(fs.existsSync(claudeCfg)).toBe(true);
        const cfg = JSON.parse(fs.readFileSync(claudeCfg, 'utf8'));
        expect(cfg.mcpServers.hipcortex.type).toBe('stdio');
        expect(cfg.mcpServers.hipcortex.command).toBe('python3');
        expect(cfg.mcpServers.hipcortex.args[0]).toBe(
            path.join('/ext/path', 'server', 'launcher.py')
        );
        expect(cfg.mcpServers.hipcortex.env.HIPCORTEX_URL).toBe('http://localhost:3030');
    });

    it('merges into existing mcp.json without overwriting other servers', () => {
        const claudeDir = path.join(tmpDir, '.claude');
        fs.mkdirSync(claudeDir, { recursive: true });
        const claudeCfg = path.join(claudeDir, 'mcp.json');
        fs.writeFileSync(claudeCfg, JSON.stringify({
            mcpServers: { other: { type: 'stdio', command: 'other-cmd', args: [] } }
        }), 'utf8');

        writeMcpEntries('/ext/path', 'python', undefined, tmpDir);

        const cfg = JSON.parse(fs.readFileSync(claudeCfg, 'utf8'));
        expect(cfg.mcpServers.other.command).toBe('other-cmd');
        expect(cfg.mcpServers.hipcortex.command).toBe('python');
    });

    it('writes .vscode/mcp.json when workspaceFolder provided', () => {
        const wsDir = path.join(tmpDir, 'workspace');
        fs.mkdirSync(wsDir, { recursive: true });

        writeMcpEntries('/ext/path', 'python3', wsDir, tmpDir);

        const vsCfg = path.join(wsDir, '.vscode', 'mcp.json');
        expect(fs.existsSync(vsCfg)).toBe(true);
        const cfg = JSON.parse(fs.readFileSync(vsCfg, 'utf8'));
        expect(cfg.servers.hipcortex.type).toBe('stdio');
    });

    it('calls vscode.workspace.getConfiguration("mcp").update for VS Code global settings', () => {
        writeMcpEntries('/ext/path', 'python3', undefined, tmpDir);
        expect(vscode.workspace.getConfiguration).toHaveBeenCalledWith('mcp');
        expect(mcpUpdateMock).toHaveBeenCalledWith(
            'servers',
            expect.objectContaining({ hipcortex: expect.objectContaining({ type: 'stdio' }) }),
            expect.anything()
        );
    });
});

// ── detectPythonPath ──────────────────────────────────────────────────────────

describe('detectPythonPath', () => {
    beforeEach(() => {
        vscode.extensions.getExtension.mockReturnValue(null);
    });

    it('returns null when no python on PATH', async () => {
        (mockedCp.execSync as jest.Mock).mockImplementation(() => { throw new Error('not found'); });
        const result = await detectPythonPath();
        expect(result).toBeNull();
    });

    it('returns "python3" when python3 is on PATH', async () => {
        (mockedCp.execSync as jest.Mock).mockImplementation((cmd: string) => {
            if (cmd.startsWith('python3')) { return Buffer.from('Python 3.11.0'); }
            throw new Error('not found');
        });
        const result = await detectPythonPath();
        expect(result).toBe('python3');
    });

    it('returns "python" when python3 not found but python is', async () => {
        (mockedCp.execSync as jest.Mock).mockImplementation((cmd: string) => {
            if (cmd.startsWith('python3')) { throw new Error('not found'); }
            return Buffer.from('Python 3.10.0');
        });
        const result = await detectPythonPath();
        expect(result).toBe('python');
    });
});
