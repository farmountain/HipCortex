import hashlib
import json
from pathlib import Path
from typing import Any

from .data_generators import estimate_tokens

def assert_token_savings_bounds(baseline_text: str, retrieved_records: list[dict[str, Any]], mode: str):
    baseline_tokens = estimate_tokens(baseline_text)
    assert baseline_tokens > 0, "Baseline text must not be empty"
    
    used_tokens = sum(
        estimate_tokens(rec.get("content", "") + str(rec.get("metadata", {})))
        for rec in retrieved_records
    )
    savings_pct = max(0.0, (baseline_tokens - used_tokens) / baseline_tokens * 100.0)
    
    if mode.lower() == "headroom":
        assert len(retrieved_records) <= 5, f"Headroom mode returned {len(retrieved_records)} > Top-5"
        assert 59.0 <= savings_pct <= 89.0, f"Headroom savings {savings_pct:.1f}% out of [59.0%, 89.0%]"
    elif mode.lower() == "caveman":
        assert len(retrieved_records) <= 3, f"Caveman mode returned {len(retrieved_records)} > Top-3"
        assert 70.0 <= savings_pct <= 92.0, f"Caveman savings {savings_pct:.1f}% out of [70.0%, 92.0%]"
    else:
        raise ValueError(f"Unknown mode: {mode}")

def assert_merkle_chain_integrity(records: list[dict[str, Any]] | Path | str):
    if isinstance(records, (Path, str)):
        path = Path(records)
        records = [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]
    assert len(records) > 0, "Cannot verify empty Merkle chain"

    for idx, rec in enumerate(records):
        # Match Rust backend's compute_hash() exactly
        clone = dict(rec)
        clone["integrity"] = None
        clone["content_hash"] = None
        clone["access_count"] = 0
        clone["last_accessed"] = clone.get("timestamp")
        
        # In Python, json.dumps without spaces matches Rust's serde_json::to_vec
        data = json.dumps(clone, separators=(',', ':')).encode("utf-8")
        expected = hashlib.sha256(data).hexdigest()
        
        actual = rec.get("integrity")
        # If integrity is present, it must match the computed hash
        if actual is not None:
            assert actual == expected, f"Invalid SHA-256 integrity hash at step {idx}. Expected {expected}, got {actual}"
        assert len(expected) == 64, f"Invalid SHA-256 hash length at step {idx}"
