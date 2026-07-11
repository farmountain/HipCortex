import httpx
from typing import Any

class HarnessHttpxClient:
    """Synchronous HTTP client wrapper with timeout hooks."""
    def __init__(self, base_url: str, timeout: float = 30.0):
        self.base_url = base_url.rstrip("/")
        self.client = httpx.Client(base_url=self.base_url, timeout=timeout)

    def post(self, endpoint: str, json: dict[str, Any] | None = None) -> httpx.Response:
        return self.client.post(endpoint, json=json)

    def get(self, endpoint: str, params: dict[str, Any] | None = None) -> httpx.Response:
        return self.client.get(endpoint, params=params)

    def delete(self, endpoint: str, params: dict[str, Any] | None = None) -> httpx.Response:
        return self.client.delete(endpoint, params=params)

def get_clients(port: int) -> HarnessHttpxClient:
    return HarnessHttpxClient(f"http://127.0.0.1:{port}")
