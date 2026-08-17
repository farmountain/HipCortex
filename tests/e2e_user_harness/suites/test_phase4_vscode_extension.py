import json
from pathlib import Path
import pytest

@pytest.mark.vscode
def test_extension_package_json_manifest():
    """Verify VS Code extension package.json structure, version, and contributed commands."""
    pkg_path = Path(__file__).parent.parent.parent.parent / "vscode-extension" / "package.json"
    assert pkg_path.exists(), "Missing vscode-extension/package.json"
    
    manifest = json.loads(pkg_path.read_text(encoding="utf-8"))
    assert manifest.get("name") == "hipcortex-memory"
    assert manifest.get("version") in ("0.5.0", "0.5.8", "0.6.0", "0.7.0")
    
    commands = {cmd["command"] for cmd in manifest.get("contributes", {}).get("commands", [])}
    assert "hipcortex.addMemory" in commands
    assert "hipcortex.queryMemory" in commands
    assert "hipcortex.healthCheck" in commands

@pytest.mark.vscode
def test_extension_api_client_endpoints():
    """Verify VS Code extension source defines and targets standard REST endpoints."""
    ext_path = Path(__file__).parent.parent.parent.parent / "vscode-extension" / "src" / "extension.ts"
    assert ext_path.exists(), "Missing vscode-extension/src/extension.ts"
    
    source = ext_path.read_text(encoding="utf-8")
    assert "HipCortexAPI" in source
    assert "/health" in source
    assert "/memory/add" in source
    assert "/memory/query" in source
    assert "http://127.0.0.1:3030" in source
