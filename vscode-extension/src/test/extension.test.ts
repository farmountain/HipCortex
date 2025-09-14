import * as assert from 'assert';
import * as vscode from 'vscode';
import axios from 'axios';
import { HipCortexAPI } from '../extension';

// Mock axios for testing
jest.mock('axios');
const mockedAxios = axios as jest.Mocked<typeof axios>;

describe('HipCortex Extension Unit Tests', () => {
    let api: HipCortexAPI;

    beforeEach(() => {
        // Reset mocks
        jest.clearAllMocks();
        
        // Mock VS Code configuration
        const mockConfig = {
            get: jest.fn((key: string, defaultValue?: any) => {
                switch (key) {
                    case 'apiUrl':
                        return 'http://localhost:3030';
                    case 'apiKey':
                        return '';
                    case 'autoStart':
                        return true;
                    default:
                        return defaultValue;
                }
            })
        };
        
        jest.spyOn(vscode.workspace, 'getConfiguration').mockReturnValue(mockConfig as any);
        
        api = new HipCortexAPI();
    });

    describe('HipCortexAPI', () => {
        test('should initialize with correct configuration', () => {
            expect(api['baseUrl']).toBe('http://localhost:3030');
            expect(api['apiKey']).toBe('');
        });

        test('should perform health check successfully', async () => {
            mockedAxios.get.mockResolvedValueOnce({ data: 'ok' });
            
            const result = await api.healthCheck();
            
            expect(result).toBe(true);
            expect(mockedAxios.get).toHaveBeenCalledWith(
                'http://localhost:3030/health',
                { timeout: 3000 }
            );
        });

        test('should handle health check failure', async () => {
            mockedAxios.get.mockRejectedValueOnce(new Error('Network error'));
            
            const result = await api.healthCheck();
            
            expect(result).toBe(false);
        });

        test('should add memory record successfully', async () => {
            const mockResponse = {
                data: {
                    success: true,
                    record_id: 'test-id-123',
                    error: null
                }
            };
            mockedAxios.post.mockResolvedValueOnce(mockResponse);
            
            const request = {
                actor: 'TestActor',
                action: 'test_action',
                target: 'test_target',
                record_type: 'Temporal'
            };
            
            const result = await api.addMemory(request);
            
            expect(result.success).toBe(true);
            expect(result.record_id).toBe('test-id-123');
            expect(mockedAxios.post).toHaveBeenCalledWith(
                'http://localhost:3030/memory/add',
                request,
                { headers: { 'Content-Type': 'application/json' } }
            );
        });

        test('should handle add memory failure', async () => {
            mockedAxios.post.mockRejectedValueOnce(new Error('API error'));
            
            const request = {
                actor: 'TestActor',
                action: 'test_action',
                target: 'test_target'
            };
            
            await expect(api.addMemory(request)).rejects.toThrow('API error');
        });

        test('should query memory records successfully', async () => {
            const mockResponse = {
                data: {
                    records: [
                        {
                            id: 'test-id-1',
                            actor: 'TestActor',
                            action: 'test_action',
                            target: 'test_target',
                            timestamp: '2025-09-13T10:00:00Z',
                            record_type: 'Temporal',
                            metadata: {}
                        }
                    ],
                    total: 1
                }
            };
            mockedAxios.get.mockResolvedValueOnce(mockResponse);
            
            const result = await api.queryMemory({ actor: 'TestActor' });
            
            expect(result.total).toBe(1);
            expect(result.records).toHaveLength(1);
            expect(result.records[0].actor).toBe('TestActor');
            expect(mockedAxios.get).toHaveBeenCalledWith(
                'http://localhost:3030/memory/query?actor=TestActor'
            );
        });

        test('should build query string correctly', async () => {
            mockedAxios.get.mockResolvedValueOnce({ data: { records: [], total: 0 } });
            
            await api.queryMemory({
                actor: 'TestActor',
                action: 'test_action',
                limit: 10
            });
            
            expect(mockedAxios.get).toHaveBeenCalledWith(
                'http://localhost:3030/memory/query?actor=TestActor&action=test_action&limit=10'
            );
        });

        test('should handle empty query parameters', async () => {
            mockedAxios.get.mockResolvedValueOnce({ data: { records: [], total: 0 } });
            
            await api.queryMemory({});
            
            expect(mockedAxios.get).toHaveBeenCalledWith(
                'http://localhost:3030/memory/query?'
            );
        });
    });

    describe('Input Validation', () => {
        test('should validate actor input', () => {
            // Test empty input
            expect(() => validateInput('', 'actor')).toThrow('actor cannot be empty');
            
            // Test whitespace-only input
            expect(() => validateInput('   ', 'actor')).toThrow('actor cannot be empty');
            
            // Test long input
            const longInput = 'a'.repeat(101);
            expect(() => validateInput(longInput, 'actor')).toThrow('actor cannot exceed 100 characters');
            
            // Test valid input
            const validInput = validateInput('ValidActor', 'actor');
            expect(validInput).toBe('ValidActor');
        });

        test('should sanitize input', () => {
            const input = 'Actor<script>alert("xss")</script>&dangerous';
            const sanitized = validateInput(input, 'actor');
            
            expect(sanitized).not.toContain('<');
            expect(sanitized).not.toContain('>');
            expect(sanitized).not.toContain('"');
            expect(sanitized).not.toContain("'");
            expect(sanitized).not.toContain('&');
        });

        test('should handle different input types', () => {
            ['actor', 'action', 'target'].forEach(type => {
                const validInput = validateInput(`Valid${type}`, type as any);
                expect(validInput).toBe(`Valid${type}`);
                
                expect(() => validateInput('', type as any)).toThrow(`${type} cannot be empty`);
            });
        });
    });

    describe('Chat Command Parsing', () => {
        test('should parse add command correctly', () => {
            const prompt = 'add actor: TestActor action: test_action target: test_target';
            
            const actorMatch = prompt.match(/actor[:\\s]+([\\w\\s]+?)(?=\\s+action|\\s+target|$)/i);
            const actionMatch = prompt.match(/action[:\\s]+([\\w\\s]+?)(?=\\s+actor|\\s+target|$)/i);
            const targetMatch = prompt.match(/target[:\\s]+([\\w\\s]+?)(?=\\s+actor|\\s+action|$)/i);
            
            expect(actorMatch![1].trim()).toBe('TestActor');
            expect(actionMatch![1].trim()).toBe('test_action');
            expect(targetMatch![1].trim()).toBe('test_target');
        });

        test('should handle partial command parsing', () => {
            const prompt = 'add actor: TestActor action: test_action';
            
            const actorMatch = prompt.match(/actor[:\\s]+([\\w\\s]+?)(?=\\s+action|\\s+target|$)/i);
            const actionMatch = prompt.match(/action[:\\s]+([\\w\\s]+?)(?=\\s+actor|\\s+target|$)/i);
            const targetMatch = prompt.match(/target[:\\s]+([\\w\\s]+?)(?=\\s+actor|\\s+action|$)/i);
            
            expect(actorMatch![1].trim()).toBe('TestActor');
            expect(actionMatch![1].trim()).toBe('test_action');
            expect(targetMatch).toBeNull();
        });

        test('should parse query command with parameters', () => {
            const prompt = 'query actor: TestActor limit: 5';
            
            const actorMatch = prompt.match(/actor[:\\s]+([\\w\\s]+?)(?=\\s+action|\\s+limit|$)/i);
            const limitMatch = prompt.match(/limit[:\\s]+(\\d+)/i);
            
            expect(actorMatch![1].trim()).toBe('TestActor');
            expect(parseInt(limitMatch![1])).toBe(5);
        });
    });

    describe('Error Handling', () => {
        test('should handle network errors gracefully', async () => {
            mockedAxios.get.mockRejectedValueOnce(new Error('ECONNREFUSED'));
            
            const result = await api.healthCheck();
            expect(result).toBe(false);
        });

        test('should handle timeout errors', async () => {
            mockedAxios.get.mockRejectedValueOnce(new Error('timeout'));
            
            const result = await api.healthCheck();
            expect(result).toBe(false);
        });

        test('should handle malformed API responses', async () => {
            mockedAxios.post.mockResolvedValueOnce({ data: 'invalid response' });
            
            const request = {
                actor: 'TestActor',
                action: 'test_action',
                target: 'test_target'
            };
            
            await expect(api.addMemory(request)).rejects.toThrow();
        });
    });

    describe('Configuration Handling', () => {
        test('should use default configuration when values are missing', () => {
            const mockConfig = {
                get: jest.fn((key: string, defaultValue?: any) => defaultValue)
            };
            
            jest.spyOn(vscode.workspace, 'getConfiguration').mockReturnValue(mockConfig as any);
            
            const newApi = new HipCortexAPI();
            expect(newApi['baseUrl']).toBe('http://127.0.0.1:3030');
            expect(newApi['apiKey']).toBe('');
        });

        test('should handle custom configuration', () => {
            const mockConfig = {
                get: jest.fn((key: string, defaultValue?: any) => {
                    switch (key) {
                        case 'apiUrl':
                            return 'http://custom-host:8080';
                        case 'apiKey':
                            return 'secret-key';
                        default:
                            return defaultValue;
                    }
                })
            };
            
            jest.spyOn(vscode.workspace, 'getConfiguration').mockReturnValue(mockConfig as any);
            
            const newApi = new HipCortexAPI();
            expect(newApi['baseUrl']).toBe('http://custom-host:8080');
            expect(newApi['apiKey']).toBe('secret-key');
        });
    });
});

// Helper function for testing (normally would be in extension.ts)
function validateInput(input: string, type: 'actor' | 'action' | 'target'): string {
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
