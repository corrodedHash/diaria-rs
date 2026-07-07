import subprocess
import uuid
from pathlib import Path

from ..helper import diaria, vault, Vault


def test_wrong_password_fails_gracefully(diaria: Path, vault: Vault, tmp_path: Path):
    """Reading an entry with the wrong password must fail (non-zero) with a
    clean, human-readable message rather than a panic/backtrace."""
    entry_text = str(uuid.uuid4())
    entry_file = tmp_path / "plaintext_entry"
    entry_file.write_text(entry_text, encoding="utf-8")

    subprocess.run(
        [diaria, "add", "--input", entry_file],
        env=vault.env,
        check=True,
    )
    [diary_file] = list(vault.entries.iterdir())

    wrong_env = {**vault.env, "DIARIA_PASSWORD": "definitely-not-the-password"}
    result = subprocess.run(
        [diaria, "read", diary_file],
        env=wrong_env,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        encoding="utf-8",
    )
    assert result.returncode != 0
    assert "failed to decrypt the private key" in result.stderr
    # The plaintext must not leak on failure.
    assert entry_text not in result.stdout
