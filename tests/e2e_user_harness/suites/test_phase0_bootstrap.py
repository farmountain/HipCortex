import pytest
from tests.e2e_user_harness.client_factory import HarnessHttpxClient

@pytest.mark.core
def test_binary_auto_build_and_health(raw_client: HarnessHttpxClient):
    resp = raw_client.get("/health")
    assert resp.status_code == 200
    data = resp.json()
    assert data.get("status") == "ok"
    assert data.get("version") == "0.4.9"
