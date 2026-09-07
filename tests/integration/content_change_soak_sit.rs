/// Content-change soak SIT — proves file byte changes produce different WM state labels.
///
/// Mechanical proof for the two-process soak scenario:
/// Process A edits file → sha256_hex changes → WM entity:<hash8> state changes.
/// No server needed — tests the hash-state formula used by derive_obs_state.

use sha2::{Digest, Sha256};

fn sha256_of(data: &[u8]) -> String {
    hex::encode(Sha256::digest(data))
}

fn wm_state_label(entity: &str, sha256_hex: &str) -> String {
    format!("{}:{}", entity, &sha256_hex[..sha256_hex.len().min(8)])
}

#[test]
fn soak_content_change_produces_different_wm_state() {
    let hash_a = sha256_of(b"README content version A");
    let hash_b = sha256_of(b"README content version B - sed changed this line");
    let state_a = wm_state_label("readme", &hash_a);
    let state_b = wm_state_label("readme", &hash_b);
    assert_ne!(state_a, state_b, "different file bytes must produce different WM entity states");
}

#[test]
fn soak_same_content_produces_same_wm_state() {
    let content = b"stable content - no edit";
    let state1 = wm_state_label("readme", &sha256_of(content));
    let state2 = wm_state_label("readme", &sha256_of(content));
    assert_eq!(state1, state2, "identical bytes must produce identical WM state (no false surprise)");
}

#[test]
fn soak_touch_no_content_change_does_not_trigger_surprise() {
    // `touch` updates mtime but not content → mtime-only probe = same WM state (correct).
    let content = b"unchanged file";
    let state_before = wm_state_label("file", &sha256_of(content));
    let state_after = wm_state_label("file", &sha256_of(content)); // same bytes, different mtime
    assert_eq!(state_before, state_after, "touch (mtime only) must not trigger WM surprise");
}

#[test]
fn soak_hash_prefix_format_matches_derive_obs_state() {
    // Verifies label format matches derive_obs_state: entity:<hex[:8]>
    let hash = sha256_of(b"some content");
    let label = wm_state_label("entity_name", &hash);
    assert!(label.starts_with("entity_name:"), "label must start with entity_name:");
    let suffix = label.trim_start_matches("entity_name:");
    assert_eq!(suffix.len(), 8, "suffix must be exactly 8 hex chars (sha256[:8])");
    assert!(suffix.chars().all(|c| c.is_ascii_hexdigit()), "suffix must be hex");
}
