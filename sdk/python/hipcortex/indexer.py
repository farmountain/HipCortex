"""
HipCortex Code Intelligence Indexer
Uses Python stdlib ast + optional tree-sitter for multi-language support.
No external deps required for Python files.
"""
from __future__ import annotations
import ast
import os
from pathlib import Path
from typing import Optional, Dict, List, Any


class CodeIndexer:
    """Walks a codebase, extracts symbols, feeds into HipCortex symbolic graph."""

    def __init__(self, client, base_url: str = "http://localhost:3030", api_key: Optional[str] = None):
        """
        Args:
            client: HipCortexClient instance
            base_url: HipCortex server URL (only needed if client not provided)
            api_key: optional API key
        """
        self._client = client
        self._node_cache: Dict[str, str] = {}  # qualified_name -> node_id

    def index(self, path: str, actor: str = "codebase",
              extensions: Optional[List[str]] = None,
              exclude_dirs: Optional[List[str]] = None) -> Dict[str, int]:
        """
        Index a directory or file into HipCortex symbolic graph.

        Returns: {"nodes": N, "edges": N, "files": N}
        """
        if extensions is None:
            extensions = [".py", ".ts", ".js"]
        if exclude_dirs is None:
            exclude_dirs = ["node_modules", ".git", "__pycache__", ".venv", "venv", "dist", "build"]

        stats = {"nodes": 0, "edges": 0, "files": 0}
        root = Path(path).resolve()

        if root.is_file():
            files = [root]
        else:
            files = [
                f for f in root.rglob("*")
                if f.suffix in extensions
                and not any(ex in f.parts for ex in exclude_dirs)
            ]

        for f in files:
            try:
                n = self._index_file(f, root, actor)
                stats["files"] += 1
                stats["nodes"] += n.get("nodes", 0)
                stats["edges"] += n.get("edges", 0)
            except Exception:
                pass  # skip unparseable files

        return stats

    def _make_node(self, label: str, props: Dict[str, str]) -> Optional[str]:
        """Create a node, return its ID. Cache by label to avoid duplicates."""
        if label in self._node_cache:
            return self._node_cache[label]
        try:
            result = self._client.create_node(label=label, properties=props)
            node_id = result.get("id")
            if node_id:
                self._node_cache[label] = node_id
            return node_id
        except Exception:
            return None

    def _make_edge(self, from_id: str, to_id: str, relation: str) -> bool:
        try:
            self._client.create_edge(from_id=from_id, to_id=to_id, relation=relation)
            return True
        except Exception:
            return False

    def _index_file(self, file: Path, root: Path, actor: str) -> Dict[str, int]:
        """Index a single file. Returns {"nodes": N, "edges": N}."""
        rel = str(file.relative_to(root))
        suffix = file.suffix

        if suffix == ".py":
            return self._index_python(file, rel, actor)
        elif suffix in (".ts", ".js"):
            return self._index_js_simple(file, rel, actor)
        return {"nodes": 0, "edges": 0}

    def _index_python(self, file: Path, rel_path: str, actor: str) -> Dict[str, int]:
        """Parse Python file with ast, extract classes + functions + imports."""
        try:
            source = file.read_text(encoding="utf-8", errors="ignore")
            tree = ast.parse(source, filename=str(file))
        except SyntaxError:
            return {"nodes": 0, "edges": 0}

        nodes = 0
        edges = 0

        # File node
        file_label = f"file:{rel_path}"
        file_id = self._make_node(file_label, {
            "type": "file", "path": rel_path, "language": "python"
        })
        if file_id:
            nodes += 1

        for node in ast.walk(tree):
            # Class definitions
            if isinstance(node, ast.ClassDef):
                label = f"class:{rel_path}:{node.name}"
                props = {
                    "type": "class",
                    "name": node.name,
                    "file": rel_path,
                    "line": str(node.lineno),
                }
                # Base classes
                bases = [ast.unparse(b) for b in node.bases if isinstance(b, (ast.Name, ast.Attribute))]
                if bases:
                    props["bases"] = ", ".join(bases[:3])

                class_id = self._make_node(label, props)
                if class_id:
                    nodes += 1
                    if file_id:
                        if self._make_edge(file_id, class_id, "defines"):
                            edges += 1
                    # Inheritance edges
                    for base in bases:
                        base_label = f"class:*:{base}"
                        # try to find existing base node
                        if base_label in self._node_cache:
                            if self._make_edge(class_id, self._node_cache[base_label], "inherits"):
                                edges += 1

            # Function / method definitions
            elif isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
                # Get parent context by checking if this is inside a class
                label = f"fn:{rel_path}:{node.name}"

                # Build signature
                args = []
                for a in node.args.args:
                    if a.annotation:
                        args.append(f"{a.arg}: {ast.unparse(a.annotation)}")
                    else:
                        args.append(a.arg)

                return_ann = ""
                if node.returns:
                    return_ann = ast.unparse(node.returns)

                props = {
                    "type": "function",
                    "name": node.name,
                    "file": rel_path,
                    "line": str(node.lineno),
                    "signature": f"def {node.name}({', '.join(args[:5])})" + (f" -> {return_ann}" if return_ann else ""),
                }
                if any(d.id == "property" if isinstance(d, ast.Name) else False for d in node.decorator_list):
                    props["is_property"] = "true"

                fn_id = self._make_node(label, props)
                if fn_id:
                    nodes += 1
                    if file_id:
                        if self._make_edge(file_id, fn_id, "defines"):
                            edges += 1

        return {"nodes": nodes, "edges": edges}

    def _index_js_simple(self, file: Path, rel_path: str, actor: str) -> Dict[str, int]:
        """Simple regex-based JS/TS indexing (no AST parser for JS in stdlib)."""
        import re
        try:
            source = file.read_text(encoding="utf-8", errors="ignore")
        except Exception:
            return {"nodes": 0, "edges": 0}

        nodes = 0
        edges = 0

        file_label = f"file:{rel_path}"
        file_id = self._make_node(file_label, {
            "type": "file", "path": rel_path,
            "language": "typescript" if rel_path.endswith(".ts") else "javascript"
        })
        if file_id:
            nodes += 1

        # Extract exported functions/classes
        patterns = [
            (r'export\s+(?:async\s+)?function\s+(\w+)', "function"),
            (r'export\s+class\s+(\w+)', "class"),
            (r'export\s+const\s+(\w+)\s*=\s*(?:async\s+)?\(', "function"),
            (r'export\s+interface\s+(\w+)', "interface"),
            (r'export\s+type\s+(\w+)', "type"),
        ]

        for pat, kind in patterns:
            for m in re.finditer(pat, source):
                name = m.group(1)
                label = f"{kind}:{rel_path}:{name}"
                sym_id = self._make_node(label, {
                    "type": kind, "name": name, "file": rel_path,
                    "line": str(source[:m.start()].count("\n") + 1),
                })
                if sym_id:
                    nodes += 1
                    if file_id:
                        if self._make_edge(file_id, sym_id, "defines"):
                            edges += 1

        return {"nodes": nodes, "edges": edges}
