import os
import shutil
import subprocess
from pathlib import Path

from .helper import diaria


def _legacy_vault(tmp_path: Path) -> Path:
    """Arrange the committed v0 example data as a legacy vault: keys + entries
    but no ``manifest.toml``, which is how a pre-versioning ("v0") vault looks."""
    src = Path(__file__).parent / "entry_data"
    vault = tmp_path / "diaria"
    (vault / "entries").mkdir(parents=True)
    for key in ["key.key", "key.pub", "key.sym"]:
        shutil.copy(src / key, vault / key)
    for entry in ["eternal.diaria", "long.diaria"]:
        shutil.copy(src / entry, vault / "entries" / entry)
    return vault


def test_old_entries_are_no_longer_supported(diaria: Path, tmp_path: Path):
    """The committed example data is a legacy, unversioned (v0) vault. Reading
    from it must be refused before any decryption, rather than decoding."""
    vault = _legacy_vault(tmp_path)
    env = {**os.environ, "XDG_DATA_HOME": str(tmp_path)}

    for entry in ["eternal.diaria", "long.diaria"]:
        result = subprocess.run(
            [diaria, "read", vault / "entries" / entry],
            env=env,
            capture_output=True,
            encoding="utf-8",
        )
        assert result.returncode != 0
        assert "predates versioning" in result.stderr
