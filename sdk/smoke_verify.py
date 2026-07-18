import sys
import time
from hipcortex import HipCortexClient

def main():
    print("--- HipCortex Local Smoke Verification ---")
    url = "http://127.0.0.1:3030"
    client = HipCortexClient(url)

    # 1. Healthcheck
    print("Testing connection...")
    if not client.health():
        print("Error: HipCortex server is not running on http://127.0.0.1:3030")
        print("Please start the server first (e.g. running cargo run)")
        sys.exit(1)
    print("✓ Connected to server")

    # 2. Add memories
    print("Adding test memories...")
    m1 = client.add_memory(
        actor="smoke_test",
        action="configured",
        target="Database set to PostgreSQL",
        record_type="Symbolic",
        metadata={"db": "postgres"}
    )
    m2 = client.add_memory(
        actor="smoke_test",
        action="validated",
        target="PostgreSQL connection string verified",
        record_type="Symbolic",
        metadata={"status": "ok"}
    )
    
    id1 = m1["record_id"]
    id2 = m2["record_id"]
    print(f"✓ Added memory 1: {id1}")
    print(f"✓ Added memory 2: {id2}")

    # 3. Link memories
    print("Linking memories...")
    link_res = client.link_memories(
        source_id=id1,
        target_id=id2,
        relation="supports"
    )
    assert link_res["success"] is True
    print("✓ Memories linked successfully")

    # 4. Get neighbors
    print("Verifying neighbors...")
    neighbors = client.get_neighbors(id1)
    assert len(neighbors) > 0, "No neighbors found"
    assert neighbors[0]["id"] == id2, f"Expected neighbor {id2}, got {neighbors[0]['id']}"
    print("✓ Neighbor verification successful")

    # 5. Search related (PPR)
    print("Verifying related memories...")
    related = client.search_related(id1)
    assert len(related) > 0, "No related memories found"
    print("✓ Related memory search successful")

    # 6. Live beliefs
    print("Verifying live beliefs...")
    beliefs = client.live_beliefs()
    assert "symbolic_facts" in beliefs, "Missing symbolic_facts in live beliefs"
    assert "current_hypotheses" in beliefs, "Missing current_hypotheses in live beliefs"
    print("✓ Live beliefs endpoint returns expected schema")

    # 7. Delete memories (cleanup)
    print("Cleaning up...")
    del_res1 = client.delete_memory(id1)
    del_res2 = client.delete_memory(id2)
    assert del_res1["success"] is True
    assert del_res2["success"] is True
    print("✓ Cleanup successful")
    
    print("-----------------------------------------")
    print("✓ ALL SMOKE TESTS PASSED SUCCESSFULLY!")
    print("-----------------------------------------")

if __name__ == "__main__":
    main()
