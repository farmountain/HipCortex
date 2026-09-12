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


INTEGRITY_FORMAT_VERSION = 1


def is_current_format(record: dict) -> bool:
    """True when the record carries an integrity hash in the current hash format.

    Two kinds of record are not current-format. Records written before the format was tagged have no
    ``hash_version`` key; their bytes were emitted by a struct layout this build no longer produces,
    so their hash can never be reproduced. Records with no ``integrity`` at all were never hashed —
    the server-side passive-capture path stores them that way. Neither is corruption.
    """
    return (
        record.get("integrity") is not None
        and record.get("hash_version", 0) >= INTEGRITY_FORMAT_VERSION
    )


def expected_integrity_hash(record: dict) -> str:
    """The hash this build would compute for ``record``. Mirrors MemoryRecord::compute_hash.

    ``ensure_ascii=False`` is required rather than stylistic: ``serde_json::to_vec`` emits raw
    UTF-8, so escaping non-ASCII here computes a different digest for any record whose text is not
    ASCII, and reports a healthy record as corrupt.
    """
    clone = dict(record)
    clone["integrity"] = None
    clone["content_hash"] = None
    clone["access_count"] = 0
    clone["last_accessed"] = clone.get("timestamp")
    # Hash the record under the format it declares, exactly as compute_hash does. Forcing the
    # current version here would turn this into a build-version check instead of a corruption check.
    # A zero tag is omitted, mirroring `skip_serializing_if = "is_zero_u32"`: keeping the key would
    # cover a byte the record was never hashed with and make every pre-tag record look tampered.
    tag = record.get("hash_version", 0)
    if tag:
        clone["hash_version"] = tag
    else:
        clone.pop("hash_version", None)
    data = json.dumps(clone, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
    return hashlib.sha256(data).hexdigest()


def count_unverifiable(records) -> int:
    """Records this build cannot verify: pre-format-tagged ones, and ones with no hash at all."""
    if isinstance(records, (Path, str)):
        records = [
            json.loads(line)
            for line in Path(records).read_text(encoding="utf-8").splitlines()
            if line.strip()
        ]
    return sum(1 for r in records if not is_current_format(r))


def assert_merkle_chain_integrity(records: list[dict[str, Any]] | Path | str) -> int:
    """Verify every current-format record; return how many cannot be verified at all.

    Records hashed by a superseded build are counted, not asserted against: their hash covered a
    struct layout this build no longer serialises, so no digest it computes can match them. Callers
    that want history to be fatal can assert on the return value.
    """
    if isinstance(records, (Path, str)):
        path = Path(records)
        records = [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]
    assert len(records) > 0, "Cannot verify empty Merkle chain"

    for idx, rec in enumerate(records):
        if not is_current_format(rec):
            continue

        expected = expected_integrity_hash(rec)

        actual = rec.get("integrity")
        assert actual == expected, f"Invalid SHA-256 integrity hash at step {idx}. Expected {expected}, got {actual}"
        assert len(expected) == 64, f"Invalid SHA-256 hash length at step {idx}"

    return count_unverifiable(records)
