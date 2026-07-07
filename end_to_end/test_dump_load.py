import subprocess
import uuid
from pathlib import Path

from .helper import diaria, vault, Vault


def test_load_dump_roundtrip(diaria: Path, vault: Vault, tmp_path: Path):
    """`load` imports a directory of plaintext files as encrypted entries;
    `dump` decrypts them back out. The dumped text must match what went in."""
    load_src = tmp_path / "load_src"
    dump_out = tmp_path / "dump_out"
    load_src.mkdir()

    # `load` stores each source file under its own name, and `dump` only emits
    # files with the `.diaria` extension (writing them out under the stem), so
    # name the sources `<stem>.diaria` to survive the round trip.
    contents = {}
    for i in range(4):
        text = str(uuid.uuid4())
        contents[f"entry_{i:02}"] = text
        (load_src / f"entry_{i:02}.diaria").write_text(text, encoding="utf-8")

    subprocess.run(
        [diaria, "load", "--directory", load_src],
        env=vault.env,
        check=True,
    )
    assert len(list(vault.entries.iterdir())) == 4

    subprocess.run(
        [diaria, "dump", "--directory", dump_out],
        env=vault.env,
        check=True,
    )

    dumped = {p.name: p.read_text(encoding="utf-8") for p in dump_out.iterdir()}
    assert dumped == contents
