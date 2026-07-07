import os
import shutil
import subprocess
from pathlib import Path

from .helper import diaria

# Password the committed entry_data_v1 vault was created with.
VAULT_PASSWORD = "password"


def _v1_vault(tmp_path: Path) -> Path:
    """Copy the committed v1 example vault into place under an isolated
    XDG_DATA_HOME so the binary resolves it as its data dir."""
    src = Path(__file__).parent / "entry_data_v1"
    vault = tmp_path / "diaria"
    shutil.copytree(src, vault)
    return vault


def test_new_entries(diaria: Path, tmp_path: Path):
    """The version 1 example data (a proper, manifest-carrying vault) must still
    decode correctly. Mirrors test_old_entries but for the supported format."""
    vault = _v1_vault(tmp_path)
    env = {
        **os.environ,
        "XDG_DATA_HOME": str(tmp_path),
        "DIARIA_PASSWORD": VAULT_PASSWORD,
    }

    read_output = subprocess.run(
        [diaria, "read", vault / "entries" / "eternal.diaria"],
        env=env,
        check=True,
        stdout=subprocess.PIPE,
        encoding="utf-8",
    ).stdout
    assert read_output.strip() == "Eons... pass like days"

    read_output = subprocess.run(
        [diaria, "read", vault / "entries" / "long.diaria"],
        env=env,
        check=True,
        stdout=subprocess.PIPE,
        encoding="utf-8",
    ).stdout
    assert (
        "This runway is covered with the last pollen from the last flowers available anywhere on Earth."
        in read_output
    )
