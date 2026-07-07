import subprocess
from pathlib import Path

from .helper import diaria, vault, Vault


def test_sync_reports_non_git_entries_dir(diaria: Path, vault: Vault):
    """A fresh vault's `entries/` is not a git repository, so `sync` reports
    that and exits successfully rather than failing."""
    result = subprocess.run(
        [diaria, "sync"],
        env=vault.env,
        stdout=subprocess.PIPE,
        encoding="utf-8",
    )
    assert result.returncode == 0
    assert "Not a git repository" in result.stdout
