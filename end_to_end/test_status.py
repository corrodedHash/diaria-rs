import os
import subprocess
from pathlib import Path

from .helper import diaria, vault, Vault


def _status(diaria: Path, env: dict[str, str]) -> str:
    return subprocess.run(
        [diaria, "status"],
        env=env,
        check=True,
        stdout=subprocess.PIPE,
        encoding="utf-8",
    ).stdout


def test_status_initialized_vault(diaria: Path, vault: Vault):
    """An initialized vault reports setup, all keys found, version 1, no entries,
    and no git sync yet."""
    out = _status(diaria, vault.env)
    assert str(vault.dir) in out
    assert str(vault.entries) in out
    assert "Vault format version: 1" in out
    assert "Setup: initialized" in out
    assert "private key:   found" in out
    assert "public key:    found" in out
    assert "symmetric key: found" in out
    assert "Entries: 0" in out
    assert "Git sync: not configured" in out


def test_status_uninitialized_vault(diaria: Path, tmp_path: Path):
    """A data home with no `init` reports not initialized and every key missing."""
    env = {**os.environ, "XDG_DATA_HOME": str(tmp_path)}
    out = _status(diaria, env)
    assert "Setup: not initialized" in out
    assert "private key:   missing" in out
    assert "public key:    missing" in out
    assert "symmetric key: missing" in out
    assert "Entries: 0" in out
    assert "Git sync: not configured" in out


def test_status_counts_entries_and_detects_git(diaria: Path, vault: Vault):
    """`status` reflects the entry count and detects a git repo under entries/."""
    (vault.entries / "2026-01-01T00:00:00.diaria").write_bytes(b"x")
    (vault.entries / "2026-01-02T00:00:00.diaria").write_bytes(b"x")
    subprocess.run(["git", "init"], cwd=vault.entries, check=True, capture_output=True)

    out = _status(diaria, vault.env)
    assert "Entries: 2" in out
    assert "Git sync: configured" in out


def test_status_is_the_default_command(diaria: Path, vault: Vault):
    """A bare `diaria` invocation (no subcommand) runs `status`."""
    out = subprocess.run(
        [diaria],
        env=vault.env,
        check=True,
        stdout=subprocess.PIPE,
        encoding="utf-8",
    ).stdout
    assert "Setup: initialized" in out
