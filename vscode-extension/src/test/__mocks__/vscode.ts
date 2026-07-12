export const StatusBarAlignment = { Right: 2, Left: 1 };

export const Uri = {
    file: (p: string) => ({ fsPath: p, toString: () => p }),
};

export const workspace = {
    getConfiguration: jest.fn(),
    name: 'test-workspace',
    onDidSaveTextDocument: jest.fn(() => ({ dispose: jest.fn() })),
};

export const window = {
    createOutputChannel: jest.fn(() => ({ appendLine: jest.fn(), show: jest.fn(), dispose: jest.fn() })),
    createStatusBarItem: jest.fn(() => ({ show: jest.fn(), dispose: jest.fn(), text: '' })),
    showInformationMessage: jest.fn(),
    showErrorMessage: jest.fn(),
    showInputBox: jest.fn(),
    showQuickPick: jest.fn(),
};

export const chat = {
    createChatParticipant: jest.fn(() => ({
        iconPath: undefined,
        followupProvider: undefined,
        dispose: jest.fn(),
    })),
};

export const commands = {
    registerCommand: jest.fn(() => ({ dispose: jest.fn() })),
};

export const lm = {
    registerTool: jest.fn(() => ({ dispose: jest.fn() })),
};

export const extensions = {
    getExtension: jest.fn(),
};

export const ExtensionContext = class {
    subscriptions: unknown[] = [];
    asAbsolutePath = (p: string) => p;
};

export type CancellationToken = { isCancellationRequested: boolean };
export type ChatRequest = { prompt: string };
export type ChatContext = unknown;
export type ChatResponseStream = { markdown: (s: string) => void };