import pytest
from tests.e2e_user_harness.client_factory import HarnessHttpxClient
from tests.e2e_user_harness.data_generators import CodingSessionTraceBuilder
from tests.e2e_user_harness.assertions import assert_token_savings_bounds

@pytest.mark.worldmodel
def test_headroom_vs_caveman_token_savings_offline_parity():
    """Verifies Headroom Mode (Top-5) vs Caveman Mode (Top-3) token savings exact formula bounds."""
    records = CodingSessionTraceBuilder.build_trace(turns=20, actor="dev_alice")
    baseline_text = "\n".join([r["content"] for r in records])
    
    # Simulate Headroom Top-5 return
    headroom_records = records[:5]
    assert_token_savings_bounds(baseline_text, headroom_records, mode="headroom")
    
    # Simulate Caveman Top-3 return
    caveman_records = records[:3]
    assert_token_savings_bounds(baseline_text, caveman_records, mode="caveman")
