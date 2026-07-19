/** @type {import('jest').Config} */
module.exports = {
    preset: 'ts-jest',
    testEnvironment: 'node',
    roots: ['<rootDir>/src/test'],
    testMatch: ['**/*.test.ts'],
    moduleNameMapper: {
        '^vscode$': '<rootDir>/src/test/__mocks__/vscode.ts',
    },
    clearMocks: true,
    // Prefer real cleanup in extension.test.ts (mock autoStartServer + deactivate + dispose).
    // forceExit left false so open handles surface; re-enable only as secondary safety if residual noise.
    forceExit: false,
};
