from dataclasses import dataclass
from pathlib import Path
import subprocess
from typing import List


@dataclass
class CommandResult:
    command: str
    returncode: int
    stdout: str
    stderr: str


class SoftwareHarness:
    """Minimal execution boundary for repository observation and validation."""

    def __init__(self, repo: str):
        self.repo = Path(repo).resolve()

    def inspect(self) -> str:
        files = [p.relative_to(self.repo).as_posix() for p in self.repo.rglob("*") if p.is_file()]
        return "\n".join(sorted(files)[:500])

    def run(self, command: str, timeout: int = 120) -> CommandResult:
        completed = subprocess.run(
            command,
            cwd=self.repo,
            shell=True,
            text=True,
            capture_output=True,
            timeout=timeout,
        )
        return CommandResult(command, completed.returncode, completed.stdout, completed.stderr)

    def validate(self, commands: List[str]) -> List[CommandResult]:
        return [self.run(command) for command in commands]
