import { HipCortexClient } from "./client";

const mockFetch = jest.fn();
global.fetch = mockFetch;

beforeEach(() => mockFetch.mockClear());

function mockResponse(data: unknown, status = 200) {
  mockFetch.mockResolvedValueOnce({
    ok: status >= 200 && status < 300,
    status,
    json: async () => data,
    text: async () => String(data),
  } as Response);
}

describe("HipCortexClient", () => {
  const client = new HipCortexClient({ baseUrl: "http://localhost:3030" });

  test("addMemory sends POST to /memory/add", async () => {
    mockResponse({ success: true, record_id: "abc-123", error: null });
    const result = await client.addMemory({ actor: "alice", action: "said", target: "hello" });
    expect(result.success).toBe(true);
    expect(result.record_id).toBe("abc-123");
    expect(mockFetch).toHaveBeenCalledWith(
      "http://localhost:3030/memory/add",
      expect.objectContaining({ method: "POST" })
    );
  });

  test("queryMemory sends GET with query params", async () => {
    mockResponse({ records: [], total: 0 });
    await client.queryMemory({ actor: "alice", limit: 10 });
    const url = mockFetch.mock.calls[0][0] as string;
    expect(url).toContain("actor=alice");
    expect(url).toContain("limit=10");
    expect(mockFetch.mock.calls[0][1].method).toBe("GET");
  });

  test("search sends POST with query body", async () => {
    mockResponse({ results: [{ score: 0.9, record: {} }], total: 1 });
    const resp = await client.search({ query: "hello", limit: 5 });
    expect(resp.results).toHaveLength(1);
    expect(resp.results[0].score).toBe(0.9);
  });

  test("forget sends DELETE to /memory/forget/:actor", async () => {
    mockResponse({ success: true, actor: "alice", records_deleted: 3, symbolic_nodes_deleted: 0, error: null });
    const result = await client.forget("alice");
    expect(result.records_deleted).toBe(3);
    expect(mockFetch.mock.calls[0][0]).toBe("http://localhost:3030/memory/forget/alice");
    expect(mockFetch.mock.calls[0][1].method).toBe("DELETE");
  });

  test("health returns true on 200", async () => {
    mockResponse("ok");
    expect(await client.health()).toBe(true);
  });

  test("health returns false on network error", async () => {
    mockFetch.mockRejectedValueOnce(new Error("ECONNREFUSED"));
    expect(await client.health()).toBe(false);
  });

  test("apiKey is sent in X-Api-Key header", async () => {
    const authClient = new HipCortexClient({ baseUrl: "http://localhost:3030", apiKey: "sk-test" });
    mockResponse({ success: true, record_id: "x", error: null });
    await authClient.addMemory({ actor: "a", action: "b", target: "c" });
    expect(mockFetch.mock.calls[0][1].headers["X-Api-Key"]).toBe("sk-test");
  });

  test("bulkAdd sends POST to /memory/bulk", async () => {
    mockResponse({ success: true, inserted: 2, failed: 0, record_ids: ["1","2"], errors: [] });
    const result = await client.bulkAdd({ records: [
      { actor: "a", action: "b", target: "c" },
      { actor: "d", action: "e", target: "f" },
    ]});
    expect(result.inserted).toBe(2);
  });

  test("liveBeliefs sends GET to /memory/live_beliefs", async () => {
    mockResponse({ summary: "Live beliefs: 0", symbolic_facts: { nodes: [] } });
    const result = await client.liveBeliefs({ limit: 5 });
    expect(result.summary).toContain("Live beliefs");
    const url = mockFetch.mock.calls[0][0] as string;
    expect(url).toContain("/memory/live_beliefs");
    expect(url).toContain("limit=5");
    expect(mockFetch.mock.calls[0][1].method).toBe("GET");
  });

  test("reflect sends POST to /memory/reflect", async () => {
    mockResponse({ hypothesis: "use postgres", confidence: 0.8, loops_run: 1 });
    const result = await client.reflect("db choice");
    expect(result.hypothesis).toBe("use postgres");
    expect(mockFetch.mock.calls[0][0]).toBe("http://localhost:3030/memory/reflect");
    expect(mockFetch.mock.calls[0][1].method).toBe("POST");
    expect(JSON.parse(mockFetch.mock.calls[0][1].body as string)).toEqual({ query: "db choice" });
  });

  test("predict sends POST to /worldmodel/predict", async () => {
    mockResponse({ from_state: "idle", action: "move", probabilities: { active: 0.9 }, entropy: 0.1 });
    const result = await client.predict({ state: "idle", action: "move" });
    expect(result.from_state).toBe("idle");
    expect(mockFetch.mock.calls[0][0]).toBe("http://localhost:3030/worldmodel/predict");
    expect(mockFetch.mock.calls[0][1].method).toBe("POST");
    expect(JSON.parse(mockFetch.mock.calls[0][1].body as string)).toEqual({
      state: "idle",
      action: "move",
    });
  });
});
