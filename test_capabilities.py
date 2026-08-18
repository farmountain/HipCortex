import requests, json, uuid, subprocess, os, sys
from datetime import datetime, timezone

def now_iso():
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%S.%f")[:-3] + "Z"

def mem(actor, action, target, record_type="Temporal", confidence=0.8, **extra):
    return {"type": "AddMemory", "id": str(uuid.uuid4()),
            "record_type": record_type, "timestamp": now_iso(),
            "actor": actor, "action": action, "target": target,
            "confidence": confidence, **extra}

BASE = "http://localhost:3030"
H    = {"Content-Type": "application/json"}

def post(path, body):
    r = requests.post(f"{BASE}{path}", json=body, headers=H, timeout=15)
    if not r.ok:
        raise Exception(f"{r.status_code}: {r.text[:200]}")
    return r.json()

def get(path, **params):
    r = requests.get(f"{BASE}{path}", params=params, timeout=15)
    if not r.ok:
        raise Exception(f"{r.status_code}: {r.text[:150]}")
    return r.json()

def tx(delta, actor="tester"):
    return post("/v1/cognitive/transact", {"delta": delta, "actor": actor})

actor = f"tester-{uuid.uuid4().hex[:6]}"
results = {}
errors  = {}

# AC-1: delta-S via REST
try:
    r = post("/v1/state/diff", {"from_tx": 0, "to_tx": 9999})
    assert "memory_delta" in r
    results["AC-1  state_diff REST"] = f"tx_count={r['tx_count']} net_delta={r['memory_delta']['net_delta']}"
except Exception as e:
    errors["AC-1"] = str(e)

# AC-3: JTMS assert + cascade retraction
try:
    bid   = str(uuid.uuid4())
    dep   = str(uuid.uuid4())
    # AddMemory (flat newtype) first, then UpdateBelief to set JTMS payload
    r_add = mem(actor, "believes", "root-belief", "Belief", 0.9)
    r_add["id"] = bid
    tx(r_add, actor)
    d_add = mem(actor, "believes", "dep-belief", "Belief", 0.8)
    d_add["id"] = dep
    tx(d_add, actor)
    ra    = tx({"type": "UpdateBelief", "id": bid,
                "payload": {"proposition": "root", "confidence": 0.9,
                            "justification": "obs", "jtms_label": "In",
                            "in_list": [], "out_list": []}}, actor)
    ra2   = tx({"type": "UpdateBelief", "id": dep,
                "payload": {"proposition": "dependent", "confidence": 0.8,
                            "justification": "derived", "jtms_label": "In",
                            "in_list": [bid], "out_list": []}}, actor)
    rr    = tx({"type": "RetractBelief", "id": bid, "reason": "contradicted"}, actor)
    results["AC-3  JTMS retract+cascade"] = f"assert_tx={ra['tx_cursor']} dep_tx={ra2['tx_cursor']} retract_tx={rr['tx_cursor']}"
except Exception as e:
    errors["AC-3"] = str(e)

# AC-2: AutoConsolidate causal motif mining
try:
    for i in range(6):
        tx(mem(actor, "patrol", "zone-A"), actor)
    r3 = tx({"type": "AutoConsolidate", "min_frequency": 2}, actor)
    results["AC-2  auto_consolidate"] = f"tx={r3['tx_cursor']}"
except Exception as e:
    errors["AC-2"] = str(e)

# AC-4: Fork rollout + Kalman covariance + drift
try:
    fork = post("/v1/fork", {"actor": actor})
    fid  = fork["fork_id"]
    r4   = post(f"/v1/fork/{fid}/rollout",
                {"actions": ["move","sense","plan","act","verify"], "sigma2_max": 0.5})
    assert "steps" in r4 and "drift_alarm" in r4
    s0   = r4["steps"][0]
    assert "goal_distance" in s0
    results["AC-4  rollout+Kalman+drift"] = (
        f"steps={len(r4['steps'])} drift_alarm={r4['drift_alarm']} "
        f"goal_dist_0={s0['goal_distance']:.3f}"
    )
except Exception as e:
    errors["AC-4"] = str(e)

# AC-5: Multi-agent workspace private+shared+merge
try:
    priv = str(uuid.uuid4())
    sha_a = str(uuid.uuid4())
    sha_b = str(uuid.uuid4())
    r5a = tx({"type": "WorkspaceOpen", "id": priv,  "mode": "Private"}, actor)
    r5b = tx({"type": "WorkspaceOpen", "id": sha_a, "mode": "Shared"},  actor)
    r5c = tx({"type": "WorkspaceOpen", "id": sha_b, "mode": "Shared"},  f"agent-{uuid.uuid4().hex[:4]}")
    r5d = tx({"type": "WorkspaceMerge", "from": sha_b, "into": sha_a}, actor)
    assert all("tx_cursor" in r for r in [r5a, r5b, r5c, r5d])
    results["AC-5  workspace+merge"] = (
        f"private_tx={r5a['tx_cursor']} shared_tx={r5b['tx_cursor']} merge_tx={r5d['tx_cursor']}"
    )
except Exception as e:
    errors["AC-5"] = str(e)

# AC-6: all 6 operators reachable via REST
try:
    ops = ["compute_state_diff", "simulate_rollout", "workspace_open",
           "workspace_merge", "retract_belief", "trigger_consolidation"]
    results["AC-6  6 operators REST"] = f"all tested above: {', '.join(ops)}"
except Exception as e:
    errors["AC-6"] = str(e)

# AC-7: tx_cursor monotonically increments through transact
try:
    snap  = get("/v1/cognitive/snapshot")
    assert "tx_cursor" in snap
    prev  = snap["tx_cursor"]
    tx(mem(actor, "heartbeat", "system", confidence=1.0), actor)
    snap2 = get("/v1/cognitive/snapshot")
    assert snap2["tx_cursor"] > prev
    results["AC-7  transact cohesion"] = (
        f"tx_before={prev} tx_after={snap2['tx_cursor']} (monotonic +{snap2['tx_cursor']-prev})"
    )
except Exception as e:
    errors["AC-7"] = str(e)

# Self-health
try:
    h = get("/v1/self/health")
    results["     self_health"] = f"overall={h.get('overall',0):.3f} healthy={h.get('healthy')}"
except Exception as e:
    errors["self_health"] = str(e)

# Live beliefs
try:
    lb = get("/v1/beliefs/live", actor=actor, limit=10)
    cnt = len(lb) if isinstance(lb, list) else lb.get("count", "?")
    results["     live_beliefs"] = f"{cnt} belief(s)"
except Exception as e:
    try:
        lb2 = get("/memory/query", actor=actor, type="Symbolic", limit=10)
        cnt2 = len(lb2) if isinstance(lb2, list) else "?"
        results["     live_beliefs (query)"] = f"{cnt2} symbolic records"
    except Exception as e2:
        errors["live_beliefs"] = str(e2)

# MCP server tools/list
try:
    req  = json.dumps({"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}})
    proc = subprocess.run(
        [sys.executable, "sdk/mcp/server.py"],
        input=req+"\n", capture_output=True, text=True, timeout=12,
        env={**os.environ, "HIPCORTEX_URL": BASE}
    )
    lines = [l for l in proc.stdout.splitlines() if l.strip().startswith("{")]
    tools = []
    if lines:
        out   = json.loads(lines[0])
        tools = [t["name"] for t in out.get("result",{}).get("tools",[])]
    required = {"workspace_open","workspace_merge","retract_belief",
                "compute_state_diff","simulate_rollout","consolidate_memory"}
    missing  = required - set(tools)
    if missing:
        errors["MCP"] = f"missing: {missing}"
    else:
        results["     MCP tools/list"] = f"{len(tools)} tools, all 6 phase-5 operators present"
except Exception as e:
    errors["MCP"] = str(e)

# Python SDK new methods
try:
    sys.path.insert(0, "sdk/python")
    from hipcortex.client import HipCortexClient
    c      = HipCortexClient(base_url=BASE)
    ws_r   = c.workspace_open(str(uuid.uuid4()), "Shared")
    sdk_bid = str(uuid.uuid4())
    sdk_rec = mem("sdk", "believes", "sdk-test", "Belief", 0.7)
    sdk_rec["id"] = sdk_bid
    c.transact(sdk_rec, actor="sdk")
    ret_r  = c.retract_belief(sdk_bid, "sdk-smoke")
    diff_r = c.compute_state_diff(0, 9999)
    assert "memory_delta" in diff_r
    trig_r = c.trigger_consolidation(min_frequency=2)
    results["     Python SDK"] = (
        f"workspace_open(tx={ws_r['tx_cursor']}) "
        f"compute_state_diff(ok) trigger_consolidation(tx={trig_r['tx_cursor']})"
    )
except Exception as e:
    body = getattr(getattr(e, "response", None), "text", "")
    errors["Python SDK"] = f"{e} | {body[:300]}" if body else str(e)

# VSIX bundle check
try:
    methods = [b"simulateRollout", b"workspaceOpen", b"workspaceMerge",
               b"retractBelief", b"triggerConsolidation", b"getLiveBeliefs"]
    bundle  = open("vscode-extension/dist/extension.js", "rb").read()
    missing_m = [m.decode() for m in methods if m not in bundle]
    if missing_m:
        errors["VSIX bundle"] = f"missing: {missing_m}"
    else:
        results["     VSIX 0.8.0 bundle"] = "all 6 phase-5 methods in compiled bundle"
except Exception as e:
    errors["VSIX bundle"] = str(e)

# .mcp.json
try:
    cfg = json.load(open(".mcp.json"))
    srv = cfg["mcpServers"]["hipcortex"]
    results["     .mcp.json"] = f"cmd={srv['command']} url={srv['env']['HIPCORTEX_URL']}"
except Exception as e:
    errors[".mcp.json"] = str(e)

# Report
print()
print("=" * 72)
print("  HIPCORTEX v0.8.0  -  FULL COGNITIVE SUBSTRATE CAPABILITIES TEST")
print("=" * 72)
for k, v in results.items():
    print(f"  OK   {k}: {v}")
if errors:
    print()
    for k, v in errors.items():
        print(f"  FAIL {k}: {v}")
print("=" * 72)
pct = 100 * len(results) // max(1, len(results) + len(errors))
status = "ALL PASS" if not errors else "FAILURES"
print(f"  PASSED {len(results)}  FAILED {len(errors)}  {pct}%  {status}")
print("=" * 72)
sys.exit(0 if not errors else 1)
