import os
import pytest
from pathlib import Path
import subprocess
import warnings
from typing import NamedTuple


# Password every `vault` fixture is initialized with.
VAULT_PASSWORD = "password"


@pytest.fixture
def diaria():
    cmake_exe_path = Path("src") / "cli" / "diaria"
    p = os.environ.get("DIARIA")
    if p is not None:
        return Path(p)

    build_dir = Path(__file__).parent.parent.parent / "build"
    if not build_dir.exists():
        raise RuntimeError(
            'Environment variable "DIARIA" not specified. Build directory not found'
        )

    if (build_dir / cmake_exe_path).exists():
        warnings.warn(
            UserWarning(
                f'Environment variable "DIARIA" not specified. Using {build_dir / cmake_exe_path}'
            )
        )

        return build_dir / cmake_exe_path
    if (build_dir / "dev" / cmake_exe_path).exists():
        warnings.warn(
            UserWarning(
                f'Environment variable "DIARIA" not specified. Using {build_dir / "dev" / cmake_exe_path}'
            )
        )

        return build_dir / "dev" / cmake_exe_path

    recursive_build_dir = [
        (x / cmake_exe_path)
        for x in build_dir.iterdir()
        if x.is_dir() and (x / cmake_exe_path).is_file()
    ]
    diaria_entry = max(
        [(x.stat().st_ctime_ns, x) for x in recursive_build_dir], key=lambda x: x[0]
    )
    warnings.warn(
        UserWarning(
            f'Environment variable "DIARIA" not specified. Using {diaria_entry[1]}'
        )
    )

    return diaria_entry[1]


class Vault(NamedTuple):
    """An initialized diaria vault an e2e test can drive.

    The whole vault is one directory under an isolated ``XDG_DATA_HOME``; the
    binary resolves it via the ``xdg`` crate. Pass ``env`` straight to
    ``subprocess.run(..., env=vault.env)`` — it carries both ``XDG_DATA_HOME``
    (so the CLI finds this vault) and ``DIARIA_PASSWORD`` (so it never prompts).
    """

    env: dict[str, str]
    # The vault directory itself: ``$XDG_DATA_HOME/diaria``.
    dir: Path
    # The ``entries/`` subdirectory holding the ``*.diaria`` files.
    entries: Path


@pytest.fixture
def vault(diaria: Path, tmp_path: Path) -> Vault:
    """Create a fresh, initialized vault under an isolated data home.

    Sets ``XDG_DATA_HOME`` to ``tmp_path`` and runs ``diaria init`` with
    ``DIARIA_PASSWORD`` set, so the returned env non-interactively drives every
    subsequent subcommand against this vault.
    """
    env = {
        **os.environ,
        "XDG_DATA_HOME": str(tmp_path),
        "DIARIA_PASSWORD": VAULT_PASSWORD,
    }
    subprocess.run([str(diaria), "init"], env=env, check=True)

    vault_dir = tmp_path / "diaria"
    return Vault(env=env, dir=vault_dir, entries=vault_dir / "entries")
