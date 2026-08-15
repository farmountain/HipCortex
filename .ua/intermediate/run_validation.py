import json, collections

graph_path = "D:/all_projects/hipcortex/.ua/intermediate/assembled-graph.json"
layers_path = "D:/all_projects/hipcortex/.ua/intermediate/layers.json"
report_path = "D:/all_projects/hipcortex/.ua/intermediate/validation-report.json"

with open(graph_path, "r", encoding="utf-8") as f:
    graph = json.load(f)
with open(layers_path, "r", encoding="utf-8") as f:
    layers_raw = json.load(f)

nodes = graph.get("nodes", [])
edges = graph.get("edges", [])
issues = []

# Build helpers
node_id_map = {}
dup_ids = []
seen_ids = set()

for n in nodes:
    nid = n.get("id", "")
    if nid in seen_ids:
        dup_ids.append(nid)
    else:
        seen_ids.add(nid)
    node_id_map[nid] = n

# 1. Every node has non-empty id
missing_id = [n for n in nodes if not n.get("id", "").strip()]
if missing_id:
    issues.append({"severity": "ERROR", "check": "node_has_id",
        "message": str(len(missing_id)) + " node(s) have missing or empty id",
        "affectedIds": [n.get("id", "<none>") for n in missing_id]})

# 2. Every node has non-empty type
missing_type = [n for n in nodes if not n.get("type", "").strip()]
if missing_type:
    issues.append({"severity": "ERROR", "check": "node_has_type",
        "message": str(len(missing_type)) + " node(s) have missing or empty type",
        "affectedIds": [n.get("id", "<none>") for n in missing_type]})

# 3. Every node has summary or description
missing_desc = []
for n in nodes:
    s = (n.get("summary") or "").strip()
    d = (n.get("description") or "").strip()
    if not s and not d:
        missing_desc.append(n.get("id", "<none>"))
if missing_desc:
    issues.append({"severity": "ERROR", "check": "node_has_summary_or_description",
        "message": str(len(missing_desc)) + " node(s) have neither summary nor description",
        "affectedIds": missing_desc})

# 4. Every edge has source and target
missing_edge_fields = []
for i, e in enumerate(edges):
    if not e.get("source", "").strip() or not e.get("target", "").strip():
        missing_edge_fields.append("edge[" + str(i) + "]:" + e.get("source", "") + "->" + e.get("target", ""))
if missing_edge_fields:
    issues.append({"severity": "ERROR", "check": "edge_has_source_and_target",
        "message": str(len(missing_edge_fields)) + " edge(s) missing source or target",
        "affectedIds": missing_edge_fields})

# 5 & 6. Dangling references
dangling_src = []
dangling_tgt = []
for e in edges:
    src = e.get("source", "")
    tgt = e.get("target", "")
    if src and src not in seen_ids:
        dangling_src.append(src)
    if tgt and tgt not in seen_ids:
        dangling_tgt.append(tgt)

if dangling_src:
    issues.append({"severity": "ERROR", "check": "edge_source_exists",
        "message": str(len(set(dangling_src))) + " unique unknown source id(s)",
        "affectedIds": list(set(dangling_src))})
if dangling_tgt:
    issues.append({"severity": "ERROR", "check": "edge_target_exists",
        "message": str(len(set(dangling_tgt))) + " unique unknown target id(s)",
        "affectedIds": list(set(dangling_tgt))})

# 7. Duplicate node ids
if dup_ids:
    issues.append({"severity": "ERROR", "check": "no_duplicate_node_ids",
        "message": str(len(dup_ids)) + " duplicate node id(s) found",
        "affectedIds": list(set(dup_ids))})

# 8. Self-referencing edges
self_ref = [e.get("source", "") for e in edges
            if e.get("source", "") == e.get("target", "") and e.get("source", "")]
if self_ref:
    issues.append({"severity": "ERROR", "check": "no_self_referencing_edges",
        "message": str(len(self_ref)) + " self-referencing edge(s)",
        "affectedIds": self_ref})

# 9. File nodes: filePath present and non-empty
file_nodes_missing_path = [n.get("id", "<none>") for n in nodes
    if n.get("type", "").lower() == "file" and not n.get("filePath", "").strip()]
if file_nodes_missing_path:
    issues.append({"severity": "ERROR", "check": "file_node_has_filepath",
        "message": str(len(file_nodes_missing_path)) + " file node(s) missing filePath",
        "affectedIds": file_nodes_missing_path})

# 10. Sparse summary (< 10 chars)
sparse_summary = []
for n in nodes:
    s = (n.get("summary") or "").strip()
    d = (n.get("description") or "").strip()
    best = s if s else d
    if 0 < len(best) < 10:
        sparse_summary.append(n.get("id", "<none>"))
if sparse_summary:
    issues.append({"severity": "WARNING", "check": "summary_too_sparse",
        "message": str(len(sparse_summary)) + " node(s) with summary/description < 10 chars",
        "affectedIds": sparse_summary})

# 11. Isolated nodes
node_in_edges = set(e.get("source", "") for e in edges) | set(e.get("target", "") for e in edges)
isolated_nodes = [n.get("id", "<none>") for n in nodes if n.get("id", "") not in node_in_edges]
if isolated_nodes:
    issues.append({"severity": "WARNING", "check": "isolated_nodes",
        "message": str(len(isolated_nodes)) + " node(s) have no edges",
        "affectedIds": isolated_nodes})

# 12. Edge type field
missing_edge_type = []
for i, e in enumerate(edges):
    if not e.get("type", "").strip():
        missing_edge_type.append("edge[" + str(i) + "]:" + e.get("source", "") + "->" + e.get("target", ""))
if missing_edge_type:
    issues.append({"severity": "WARNING", "check": "edge_has_type",
        "message": str(len(missing_edge_type)) + " edge(s) missing type field",
        "affectedIds": missing_edge_type})

# 13 & 14. Layer coverage
layer_node_ids = set()
for layer in layers_raw:
    for nid in layer.get("nodeIds", []):
        layer_node_ids.add(nid)

covered_count = len(layer_node_ids & seen_ids)
total_count = len(seen_ids)
uncovered = sorted(seen_ids - layer_node_ids)

if uncovered:
    issues.append({"severity": "WARNING", "check": "layer_coverage",
        "message": str(len(uncovered)) + " node(s) do not appear in any layer",
        "affectedIds": uncovered})

layer_pct = round(covered_count / total_count * 100, 1) if total_count else 0.0

# Stats
node_type_dist = dict(collections.Counter(n.get("type", "<missing>") for n in nodes))
edge_type_dist = dict(collections.Counter(e.get("type", "<missing>") for e in edges))

files_missing_fp = [n.get("id", "<none>") for n in nodes
                    if "filePath" in n and not n.get("filePath", "").strip()]

nodes_missing_summary = missing_desc

# Determine status
has_errors = any(i["severity"] == "ERROR" for i in issues)
has_warnings = any(i["severity"] == "WARNING" for i in issues)

if has_errors:
    status = "FAIL"
elif has_warnings:
    status = "PASS_WITH_WARNINGS"
else:
    status = "PASS"

error_count = sum(1 for i in issues if i["severity"] == "ERROR")
warning_count = sum(1 for i in issues if i["severity"] == "WARNING")
info_count = sum(1 for i in issues if i["severity"] == "INFO")

summary_line = (
    status + ": " + str(total_count) + " nodes, " + str(len(edges)) + " edges — " +
    str(error_count) + " error(s), " + str(warning_count) + " warning(s), " +
    str(info_count) + " info(s). Layer coverage " + str(layer_pct) + "%."
)

report = {
    "status": status,
    "stats": {
        "totalNodes": total_count,
        "totalEdges": len(edges),
        "nodeTypeDistribution": node_type_dist,
        "edgeTypeDistribution": edge_type_dist,
        "layerCoverage": {
            "covered": covered_count,
            "total": total_count,
            "percentage": layer_pct
        },
        "filesMissingFilePath": files_missing_fp,
        "nodesMissingSummary": nodes_missing_summary,
        "danglingEdgeReferences": list(set(dangling_src + dangling_tgt)),
        "duplicateNodeIds": list(set(dup_ids)),
        "isolatedNodesCount": len(isolated_nodes)
    },
    "issues": issues,
    "summary": summary_line
}

with open(report_path, "w", encoding="utf-8") as f:
    json.dump(report, f, indent=2)

print("Report written to:", report_path)
print("Status:", status)
print("Summary:", summary_line)
print()
print("=== ISSUES ===")
for iss in issues:
    print("[" + iss["severity"] + "] " + iss["check"] + ": " + iss["message"])
    if iss["affectedIds"]:
        print("   First 5 affected:", iss["affectedIds"][:5])
print()
print("Node type distribution:", node_type_dist)
print("Edge type distribution:", edge_type_dist)
print("Layer coverage:", covered_count, "/", total_count, "(" + str(layer_pct) + "%)")
