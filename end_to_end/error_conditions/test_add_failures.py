import os
import stat
import subprocess
import uuid
from pathlib import Path

from ..helper import diaria, vault, Vault


def test_add_with_missing_public_key(diaria: Path, vault: Vault, tmp_path: Path):
    """`add` must fail when `key.pub` is missing, and no entry may be stored."""
    vault.dir.joinpath("key.pub").unlink()

    entry_text = str(uuid.uuid4())
    entry_file = tmp_path / "plaintext_entry"
    entry_file.write_text(entry_text, encoding="utf-8")

    result = subprocess.run(
        [diaria, "add", "--input", entry_file],
        env=vault.env,
        stderr=subprocess.PIPE,
        encoding="utf-8",
    )
    assert result.returncode != 0
    assert list(vault.entries.iterdir()) == []


def test_add_with_corrupt_symmetric_key(diaria: Path, vault: Vault, tmp_path: Path):
    """`add` must fail when `key.sym` is corrupt, and no entry may be stored."""
    # Truncate the symmetric key so it's too short.
    vault.dir.joinpath("key.sym").write_bytes(b"\x00")

    entry_text = str(uuid.uuid4())
    entry_file = tmp_path / "plaintext_entry"
    entry_file.write_text(entry_text, encoding="utf-8")

    result = subprocess.run(
        [diaria, "add", "--input", entry_file],
        env=vault.env,
        stderr=subprocess.PIPE,
        encoding="utf-8",
    )
    assert result.returncode != 0
    assert list(vault.entries.iterdir()) == []


def test_add_with_readonly_entries_dir(diaria: Path, vault: Vault, tmp_path: Path):
    """`add` must fail when the entries directory is not writable, and no
    entry may be stored. The original input file must be preserved."""
    # Remove write permission from the entries directory.
    entries_dir = vault.dir / "entries"
    # Collect entries that already exist.
    before = sorted(entries_dir.iterdir())
    entries_dir.chmod(stat.S_IRUSR | stat.S_IXUSR)

    entry_text = str(uuid.uuid4())
    entry_file = tmp_path / "plaintext_entry"
    entry_file.write_text(entry_text, encoding="utf-8")

    result = subprocess.run(
        [diaria, "add", "--input", entry_file],
        env=vault.env,
        stderr=subprocess.PIPE,
        encoding="utf-8",
    )
    assert result.returncode != 0

    # Restore permissions so cleanup doesn't fail.
    entries_dir.chmod(stat.S_IRWXU)

    # No new entries were created.
    assert sorted(entries_dir.iterdir()) == before
    # The original input file is intact.
    assert entry_file.read_text(encoding="utf-8") == entry_text
