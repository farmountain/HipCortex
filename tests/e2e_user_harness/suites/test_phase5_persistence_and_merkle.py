from pathlib import Path
import pytest
from tests.e2e_user_harness.client_factory import HarnessHttpxClient
from tests.e2e_user_harness.server_manager import HipCortexServerManager
from tests.e2e_user_harness.assertions import assert_merkle_chain_integrity

@pytest.mark.persistence
def test_merkle_chain_audit_integrity(raw_client: HarnessHttpxClient, hipcortex_server: HipCortexServerManager):
    """Verify write-once append log and Merkle chain hash continuity."""
    for i in range(5):
        payload = {
            "actor": "audit_bot",
            "action": f"merkle_step_{i}",
            "target": f"src/file_{i}.rs",
            "content": f"Verifying cryptographic hash linkage for step {i}",
            "metadata": {"step": i}
        }
        resp = raw_client.post("/memory/add", json=payload)
        assert resp.status_code in (200, 201)
        
    storage_dir = hipcortex_server.storage_dir
    memory_file = storage_dir / "memory.jsonl"
    assert memory_file.exists(), f"Expected memory.jsonl in {storage_dir}"
    
    assert_merkle_chain_integrity(memory_file)

@pytest.mark.persistence
def test_server_restart_persistence(raw_client: HarnessHttpxClient, hipcortex_server: HipCortexServerManager):
    """Verify data persistence across process termination and startup cycles."""
    # Write unique canary record
    canary_payload = {
        "actor": "persistence_canary",
        "action": "reboot_check",
        "target": "SYSTEM_STATE - Surviving process termination and restart",
        "metadata": {"content": "Surviving process termination and restart"},

    }
    raw_client.post("/memory/add", json=canary_payload)
    
    # Check that record is currently queryable
    q_resp = raw_client.get("/memory/query?actor=persistence_canary")
    assert q_resp.status_code == 200
    assert len(q_resp.json()) >= 1
    
    # Store critical paths before shutting down server
    storage_dir = hipcortex_server.storage_dir
    binary_path = hipcortex_server.binary_path
    
    # Stop existing server (without deleting storage)
    hipcortex_server.stop(remove_storage=False)
    raw_client.client.close()

    
    # Launch new instance pointing to identical storage_dir
    new_manager = HipCortexServerManager(build_from_source=False)
    new_manager.binary_path = binary_path
    new_manager.storage_dir = storage_dir
    new_manager.start()
    
    try:
        from hipcortex import HipCortexClient
        rebooted_client = HipCortexClient(f"http://127.0.0.1:{new_manager.port}")
        assert rebooted_client.health() is True
        
        records = rebooted_client.query_memory(actor="persistence_canary")
        assert len(records) >= 1
        assert any("Surviving process termination" in r.get("target", "") or "Surviving process termination" in r.get("content", "") for r in records)
    finally:
        new_manager.stop(remove_storage=False)
        # Re-start hipcortex_server so downstream teardown fixtures remain clean
        hipcortex_server.start()

