# Synthetic fixture — a minimal mcp_server.py that predates the _req fix.
#
# In the pre-fix server, dispatched handlers called _req("POST", ...) but
# _req was not defined at module scope: only _post/_get/_delete existed.
# This file preserves that shape so test_mcp_tool_surface.py can prove
# _unresolved_globals catches the defect class.
#
# DO NOT "fix" this file — the unresolved _req call is intentional.

import json  # noqa: F401

TOOLS = [
    {
        "name": "add_memory",
        "inputSchema": {
            "required": ["content"],
            "properties": {"content": {}},
        },
    }
]


def dispatch_tool(name, args):
    handlers = {
        "add_memory": handle_add_memory,
    }
    fn = handlers.get(name)
    if fn is None:
        return json.dumps({"error": f"unknown tool: {name}"})
    return fn(args)


def handle_add_memory(args):
    # _req is intentionally absent — this is the defect the test validates.
    result = _req("POST", "/memory/add", {"content": args["content"]})  # noqa: F821
    return json.dumps(result)
