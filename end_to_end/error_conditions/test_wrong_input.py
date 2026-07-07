import subprocess
from pathlib import Path

from ..helper import diaria, vault, Vault


def test_missing_input_file_is_rejected(diaria: Path, vault: Vault):
    """`add -i <nonexistent>` must fail and leave the vault untouched."""
    result = subprocess.run(
        [diaria, "add", "--input", "/nonexistent/path/does-not-exist"],
        env=vault.env,
        stderr=subprocess.PIPE,
        encoding="utf-8",
    )
    assert result.returncode != 0
    assert list(vault.entries.iterdir()) == []
