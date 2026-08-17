import pytest
from tests.e2e_user_harness.client_factory import HarnessHttpxClient

@pytest.mark.core
def test_binary_auto_build_and_health(raw_client: HarnessHttpxClient):
    resp = raw_client.get("/health")
    assert resp.status_code == 200
    data = resp.json()
    assert data.get("status") == "ok"
    assert data.get("version") in ("0.5.2", "0.5.8", "0.6.0", "0.7.0", "0.8.0")
