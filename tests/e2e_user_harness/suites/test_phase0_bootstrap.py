import pytest
from tests.e2e_user_harness.client_factory import HarnessHttpxClient
from tests.e2e_user_harness.repo_version import repo_version

@pytest.mark.core
def test_binary_auto_build_and_health(raw_client: HarnessHttpxClient):
    resp = raw_client.get("/health")
    assert resp.status_code == 200
    data = resp.json()
    assert data.get("status") == "ok"
    expected = repo_version()
    assert data.get("version") == expected, (
        f"/health reports {data.get('version')!r} but the repo VERSION file says "
        f"{expected!r} - the running server is stale relative to the source tree"
    )
