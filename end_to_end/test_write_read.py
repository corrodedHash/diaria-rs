import subprocess
import uuid
from pathlib import Path

from .helper import diaria, vault, Vault


def test_write_read_roundtrip(diaria: Path, vault: Vault, tmp_path: Path):
    """`add -i <file>` encrypts one entry into the vault; reading it back yields
    the original text verbatim."""
    entry_text = str(uuid.uuid4())
    entry_file = tmp_path / "plaintext_entry"
    entry_file.write_text(entry_text, encoding="utf-8")

    subprocess.run(
        [diaria, "add", "--input", entry_file],
        env=vault.env,
        check=True,
    )

    # `add` stores exactly one entry, named after its creation timestamp.
    [diary_file] = list(vault.entries.iterdir())

    read_output = subprocess.run(
        [diaria, "read", diary_file],
        env=vault.env,
        check=True,
        stdout=subprocess.PIPE,
        encoding="utf-8",
    ).stdout
    assert read_output.strip() == entry_text
