import pytest
import shutil
from pathlib import Path
from .server_manager import HipCortexServerManager
from .client_factory import HarnessHttpxClient, get_clients

@pytest.fixture(scope="session")
def hipcortex_binary():
    # Build once per pytest session
    mgr = HipCortexServerManager(build_from_source=True)
    return mgr.binary_path

@pytest.fixture(scope="class")
def hipcortex_server(hipcortex_binary):
    mgr = HipCortexServerManager(build_from_source=False)
    with mgr.running(remove_storage_on_clean_exit=True) as running_mgr:
        yield running_mgr

@pytest.fixture
def raw_client(hipcortex_server: HipCortexServerManager) -> HarnessHttpxClient:
    return get_clients(hipcortex_server.port)
