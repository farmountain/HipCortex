import subprocess
import time
import socket
import tempfile
import shutil
import os
import psutil
from pathlib import Path
from contextlib import contextmanager
import tenacity
import urllib.request
import json

class HipCortexServerManager:
    """Manages isolated local Rust webserver instances with dynamic ports and temp storage."""
    
    def __init__(self, port: int | None = None, storage_dir: Path | None = None, build_from_source: bool = True):
        self.port = port or self._find_free_port()
        self.storage_dir = storage_dir or Path(tempfile.mkdtemp(prefix=f"hipcortex_test_{self.port}_"))
        self.process: subprocess.Popen | None = None
        self.log_file = self.storage_dir / "server.log"
        self.binary_path = self._resolve_binary(build_from_source)
        
    @staticmethod
    def _find_free_port() -> int:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            s.bind(("127.0.0.1", 0))
            return s.getsockname()[1]

    def _resolve_binary(self, build_from_source: bool) -> Path:
        root_dir = Path(__file__).resolve().parent.parent.parent
        binary = root_dir / "target" / "debug" / "webserver"
        if os.name == "nt":
            binary = binary.with_suffix(".exe")
            
        if build_from_source:
            cmd = ["cargo", "build", "--no-default-features", "--bin", "webserver", "--features", "web-server,petgraph_backend"]
            try:
                subprocess.run(cmd, cwd=root_dir, check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            except subprocess.CalledProcessError:
                if not binary.exists():
                    raise
        return binary if binary.exists() else Path(shutil.which("hipcortex") or "hipcortex")

    @tenacity.retry(stop=tenacity.stop_after_attempt(20), wait=tenacity.wait_fixed(0.3))
    def wait_healthy(self) -> dict:
        url = f"http://127.0.0.1:{self.port}/health"
        req = urllib.request.Request(url, headers={"Accept": "application/json"})
        with urllib.request.urlopen(req, timeout=2) as resp:
            data = json.loads(resp.read().decode("utf-8"))
            if resp.status == 200 and data.get("status") == "ok":
                return data
        raise RuntimeError("Health check check failed")

    def start(self):
        self.storage_dir.mkdir(parents=True, exist_ok=True)
        env = os.environ.copy()
        env["HIPCORTEX_STORAGE"] = str(self.storage_dir)
        env["DATA_DIR"] = str(self.storage_dir)
        env["PORT"] = str(self.port)
        env["RUST_LOG"] = "info"
        
        cmd = [str(self.binary_path), "--port", str(self.port)]
        log_handle = open(self.log_file, "w")
        self.process = subprocess.Popen(cmd, stdout=log_handle, stderr=subprocess.STDOUT, env=env)
        try:
            self.wait_healthy()
        except Exception:
            self.stop()
            raise
        return self

    def stop(self, remove_storage: bool = False):
        if self.process and self.process.poll() is None:
            try:
                parent = psutil.Process(self.process.pid)
                children = parent.children(recursive=True)
                for child in children:
                    try:
                        child.terminate()
                    except psutil.NoSuchProcess:
                        pass
                try:
                    parent.terminate()
                    parent.wait(timeout=2)
                except (psutil.NoSuchProcess, psutil.TimeoutExpired):
                    pass
                for child in children:
                    try:
                        if child.is_running():
                            child.kill()
                    except psutil.NoSuchProcess:
                        pass
                try:
                    if parent.is_running():
                        parent.kill()
                except psutil.NoSuchProcess:
                    pass
            except psutil.NoSuchProcess:
                pass
            try:
                if self.process.poll() is None:
                    self.process.kill()
            except Exception:
                pass
            self.process = None

        if remove_storage and self.storage_dir.exists():
            shutil.rmtree(self.storage_dir, ignore_errors=True)

    @contextmanager
    def running(self, remove_storage_on_clean_exit: bool = True):
        self.start()
        try:
            yield self
        finally:
            self.stop(remove_storage=remove_storage_on_clean_exit)
