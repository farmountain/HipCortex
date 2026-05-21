"""HipCortex Gradio Demo — deployable to HuggingFace Spaces.

Run locally:
    pip install gradio requests
    HIPCORTEX_URL=https://hipcortex.fly.dev python sdk/gradio/app.py
    # or with local server: HIPCORTEX_URL=http://localhost:3030

Deploy to HuggingFace Spaces:
    1. Create a new Space at huggingface.co/new-space
    2. Choose Gradio SDK
    3. Upload this file as app.py
    4. Add HIPCORTEX_URL secret in Space settings
"""

import os
import requests
import gradio as gr

HIPCORTEX_URL = os.getenv("HIPCORTEX_URL", "https://hipcortex.fly.dev")
API_KEY = os.getenv("HIPCORTEX_API_KEY", "")

def _headers():
    h = {"Content-Type": "application/json"}
    if API_KEY:
        h["X-Api-Key"] = API_KEY
    return h

def check_health():
    try:
        r = requests.get(f"{HIPCORTEX_URL}/health", timeout=5)
        return "✅ Connected" if r.status_code == 200 else f"⚠️ Status {r.status_code}"
    except Exception as e:
        return f"❌ {e}"

def get_stats():
    try:
        r = requests.get(f"{HIPCORTEX_URL}/stats", timeout=5)
        d = r.json()
        lines = [
            f"**Total records:** {d.get('total_records', 0)}",
            f"**Unique actors:** {d.get('unique_actors', 0)}",
            f"**Metering:** {'enabled' if d.get('metering_enabled') else 'open mode'}",
        ]
        by_type = d.get("by_type", {})
        if by_type:
            lines.append("**By type:**")
            for t, count in sorted(by_type.items()):
                lines.append(f"  - {t}: {count}")
        return "\n".join(lines)
    except Exception as e:
        return f"Error: {e}"

def add_memory(actor, action, target, ttl_days):
    if not actor or not target:
        return "⚠️ Actor and target are required."
    body = {"actor": actor, "action": action or "noted", "target": target}
    if ttl_days and ttl_days > 0:
        body["ttl_seconds"] = int(ttl_days * 86400)
    try:
        r = requests.post(f"{HIPCORTEX_URL}/memory/add", json=body, headers=_headers(), timeout=10)
        d = r.json()
        if d.get("success"):
            return f"✅ Stored (id: {d.get('record_id', '?')})"
        return f"❌ {d.get('error', 'unknown error')}"
    except Exception as e:
        return f"Error: {e}"

def search_memory(query, actor_filter, limit):
    if not query:
        return "⚠️ Enter a search query."
    try:
        params = f"?query={requests.utils.quote(query)}&limit={int(limit or 10)}"
        if actor_filter:
            params += f"&actor={requests.utils.quote(actor_filter)}"
        r = requests.get(f"{HIPCORTEX_URL}/memory/search-flat{params}", timeout=10)
        d = r.json()
        memories = d.get("memories", [])
        if not memories:
            return "No memories found."
        return "\n".join(f"{i+1}. {m}" for i, m in enumerate(memories))
    except Exception as e:
        return f"Error: {e}"

def export_memories(actor_filter):
    try:
        url = f"{HIPCORTEX_URL}/memory/export"
        if actor_filter:
            url += f"?actor={requests.utils.quote(actor_filter)}"
        r = requests.get(url, headers=_headers(), timeout=15)
        d = r.json()
        records = d.get("records", [])
        if not records:
            return "No records to export."
        lines = [f"Exported {len(records)} records (at {d.get('exported_at','?')}):\n"]
        for rec in records[:50]:  # Show max 50 in UI
            lines.append(f"[{rec['action']}] {rec['target']} — actor: {rec['actor']}")
        if len(records) > 50:
            lines.append(f"... and {len(records)-50} more")
        return "\n".join(lines)
    except Exception as e:
        return f"Error: {e}"

def forget_actor(actor):
    if not actor:
        return "⚠️ Enter an actor name."
    try:
        r = requests.delete(f"{HIPCORTEX_URL}/memory/forget/{actor}", headers=_headers(), timeout=10)
        d = r.json()
        return f"✅ Deleted {d.get('records_deleted', 0)} records for '{actor}'"
    except Exception as e:
        return f"Error: {e}"

# ─── Gradio UI ────────────────────────────────────────────────────────────────

with gr.Blocks(title="HipCortex Memory Demo", theme=gr.themes.Soft()) as demo:
    gr.Markdown("""
# 🧠 HipCortex Memory Engine
**Persistent causal memory for AI agents** — [GitHub](https://github.com/farmountain/HipCortex) · [Docs](https://hipcortex.fly.dev/openapi.json)

0.6ms write latency · Temporal decay · Merkle-chained audit log · GDPR right-to-forget · MCP for Cursor/Claude Code
""")

    with gr.Row():
        health_btn = gr.Button("🔌 Check connection", variant="secondary", scale=1)
        health_out = gr.Textbox(label="Status", scale=3, interactive=False)
        health_btn.click(check_health, outputs=health_out)

    with gr.Tab("📝 Add Memory"):
        with gr.Row():
            actor_in = gr.Textbox(label="Actor (scope)", placeholder="my-project", scale=2)
            action_in = gr.Textbox(label="Action", placeholder="decided", value="noted", scale=1)
        target_in = gr.Textbox(label="Memory content", placeholder="We chose PostgreSQL for multi-user support", lines=3)
        ttl_in = gr.Slider(label="Auto-expire (days, 0 = permanent)", minimum=0, maximum=365, value=0, step=1)
        add_btn = gr.Button("💾 Store memory", variant="primary")
        add_out = gr.Textbox(label="Result", interactive=False)
        add_btn.click(add_memory, inputs=[actor_in, action_in, target_in, ttl_in], outputs=add_out)

    with gr.Tab("🔍 Search"):
        query_in = gr.Textbox(label="Search query", placeholder="authentication decisions")
        with gr.Row():
            actor_filter_in = gr.Textbox(label="Filter by actor (optional)", scale=2)
            limit_in = gr.Slider(label="Max results", minimum=1, maximum=50, value=10, step=1, scale=1)
        search_btn = gr.Button("🔎 Search", variant="primary")
        search_out = gr.Textbox(label="Results", lines=10, interactive=False)
        search_btn.click(search_memory, inputs=[query_in, actor_filter_in, limit_in], outputs=search_out)

    with gr.Tab("📊 Stats & Export"):
        with gr.Row():
            stats_btn = gr.Button("📈 Get stats", variant="secondary")
            stats_out = gr.Markdown()
        stats_btn.click(get_stats, outputs=stats_out)

        gr.Markdown("---")
        export_actor = gr.Textbox(label="Filter by actor (optional)", placeholder="Leave blank for all")
        export_btn = gr.Button("📤 Export records", variant="secondary")
        export_out = gr.Textbox(label="Export", lines=15, interactive=False)
        export_btn.click(export_memories, inputs=[export_actor], outputs=export_out)

    with gr.Tab("🗑️ GDPR Forget"):
        gr.Markdown("⚠️ **Irreversible.** Deletes all memories for an actor from temporal store, symbolic graph, and audit log.")
        forget_actor_in = gr.Textbox(label="Actor to forget", placeholder="user-42")
        forget_btn = gr.Button("🗑️ Forget actor", variant="stop")
        forget_out = gr.Textbox(label="Result", interactive=False)
        forget_btn.click(forget_actor, inputs=[forget_actor_in], outputs=forget_out)

    gr.Markdown("""
---
**HipCortex** is open source (Apache 2.0) · [GitHub](https://github.com/farmountain/HipCortex) · [Live demo](https://hipcortex.fly.dev) · [Pricing](https://hipcortex.fly.dev/pricing)

To use your own instance: set `HIPCORTEX_URL` and optionally `HIPCORTEX_API_KEY` as Space secrets.
""")

if __name__ == "__main__":
    demo.launch(server_port=7860, share=False)
