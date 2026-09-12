"""The Merkle assertion must still be able to fail: split for legacy records, strict for current
ones. Without this, Task 4's change would be indistinguishable from deleting the check."""
import hashlib
import json

import pytest

from tests.e2e_user_harness.assertions import (
    assert_merkle_chain_integrity,
    count_unverifiable,
    expected_integrity_hash,
)


def _record(integrity_ok: bool) -> dict:
    rec = {
        "id": "00000000-0000-0000-0000-000000000001",
        "record_type": "Temporal",
        "timestamp": "2026-09-12T00:00:00Z",
        "actor": "merkle-strictness",
        "action": "test",
        "target": "t",
        "metadata": {},
        "integrity": None,
        "content_hash": None,
        "access_count": 0,
        "last_accessed": "2026-09-12T00:00:00Z",
        "relevance_score": 1.0,
        "expires_at": None,
        "confidence": 1.0,
        "source": None,
        "version": 0,
        "tags": [],
        "priority": "normal",
        "status": "active",
        "hash_version": 1,
    }
    rec["integrity"] = expected_integrity_hash(rec)
    if not integrity_ok:
        rec["target"] = "changed after hashing"
    return rec


def test_current_format_record_is_verified():
    assert assert_merkle_chain_integrity([_record(True)]) == 0


def test_current_format_tampering_fails():
    with pytest.raises(AssertionError):
        assert_merkle_chain_integrity([_record(False)])


def test_pre_tag_record_is_counted_not_failed():
    rec = _record(False)
    del rec["hash_version"]
    assert assert_merkle_chain_integrity([rec]) == 1
    assert count_unverifiable([rec]) == 1


def test_pre_tag_hash_omits_the_tag_key():
    """Pin the omission rule: writing ``"hash_version": 0`` instead of dropping the key changes the
    digest, which is what made a first attempt at this measurement report 0 verifiable records on a
    store where 436 of them do reproduce."""
    rec = _record(True)
    del rec["hash_version"]

    clone = dict(rec)
    clone["integrity"] = None
    clone["content_hash"] = None
    clone["access_count"] = 0
    clone["last_accessed"] = clone["timestamp"]
    without_key = hashlib.sha256(
        json.dumps(clone, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
    ).hexdigest()

    with_zero_key = dict(clone)
    with_zero_key["hash_version"] = 0
    with_key = hashlib.sha256(
        json.dumps(with_zero_key, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
    ).hexdigest()

    assert without_key != with_key, "the two forms must differ, or this test documents nothing"
    assert expected_integrity_hash(rec) == without_key


def test_record_with_no_hash_is_counted_not_failed():
    """The passive-capture path stores records with ``integrity: null``; they are unverifiable, not
    tampered. Half of a fresh store is written by that path, so this bucket is not hypothetical."""
    rec = _record(True)
    rec["integrity"] = None
    assert assert_merkle_chain_integrity([rec]) == 1
    assert count_unverifiable([rec]) == 1


def test_non_ascii_record_is_hashed_as_raw_utf8():
    """Why expected_integrity_hash passes ensure_ascii=False.

    ``serde_json::to_vec`` emits raw UTF-8, so escaping non-ASCII here would compute a different
    digest for every record whose text is not ASCII and report healthy records as corrupt. The
    assertion below is made against a digest this test builds itself, so it pins the helper to the
    raw-UTF-8 convention rather than merely to itself.
    """
    rec = _record(True)
    rec["target"] = "café — naïve"

    clone = dict(rec)
    clone["integrity"] = None
    clone["content_hash"] = None
    clone["access_count"] = 0
    clone["last_accessed"] = clone["timestamp"]
    clone["hash_version"] = 1

    raw = hashlib.sha256(
        json.dumps(clone, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
    ).hexdigest()
    escaped = hashlib.sha256(
        json.dumps(clone, separators=(",", ":"), ensure_ascii=True).encode("utf-8")
    ).hexdigest()
    assert raw != escaped, "the two conventions must differ, or this test documents nothing"

    assert expected_integrity_hash(rec) == raw
    rec["integrity"] = raw
    assert assert_merkle_chain_integrity([rec]) == 0
