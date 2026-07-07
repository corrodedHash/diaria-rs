import subprocess
from pathlib import Path

import pytest

from .helper import diaria, vault, Vault


@pytest.mark.parametrize("whitespace", ["", " ", "\t", "\n", "\r", " \t\n\r"])
def test_empty_or_whitespace_entry_is_rejected(
    diaria: Path, vault: Vault, tmp_path: Path, whitespace: str
):
    """`add` refuses an empty or whitespace-only entry before any crypto, so no
    blank entry is ever stored."""
    entry_file = tmp_path / "blank_entry"
    entry_file.write_text(whitespace, encoding="utf-8")

    result = subprocess.run(
        [diaria, "add", "--input", entry_file],
        env=vault.env,
        stderr=subprocess.PIPE,
        encoding="utf-8",
    )

    assert result.returncode != 0
    assert "refusing to add an empty entry" in result.stderr
    # Nothing may have been written to the vault.
    assert list(vault.entries.iterdir()) == []
