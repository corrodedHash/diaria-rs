import subprocess
from pathlib import Path

from .helper import diaria, vault, Vault


def _row_count(stdout: str) -> int:
    """`stats` renders a fixed seven-row block (one row per weekday) for every
    year in the inclusive span from the earliest to the latest entry. Each row
    is newline-terminated and `print` adds one final newline, so the number of
    grid rows is ``stdout.count("\\n") - 1``."""
    return stdout.count("\n") - 1


def test_stats_buckets_by_year(diaria: Path, vault: Vault):
    """`stats` reads only each entry filename's timestamp and size (no keys, no
    decryption), so undecryptable dummy files suffice. Entries in 2019 and 2021
    span three years (2019, 2020, 2021), but 2020 is empty so it is not rendered
    → 2 * 9 rows (year header, month header, 7 weekday rows). Trailing empty
    weeks within each year are also trimmed."""
    (vault.entries / "2019-03-04T08:00:00.diaria").write_bytes(b"x")
    (vault.entries / "2021-11-02T08:00:00.diaria").write_bytes(b"x")

    stats_output = subprocess.run(
        [diaria, "stats"],
        env=vault.env,
        check=True,
        stdout=subprocess.PIPE,
        encoding="utf-8",
    ).stdout

    assert _row_count(stats_output) == 2 * 9
    # A rendered cell proves the entries were actually counted (before the
    # timestamp-parsing fix, stats produced no output at all).
    assert "0" in stats_output


def test_stats_single_year(diaria: Path, vault: Vault):
    """A single year's entries produce exactly one nine-row block (year header,
    month header, 7 weekday rows)."""
    (vault.entries / "2020-06-06T08:00:00.diaria").write_bytes(b"x")

    stats_output = subprocess.run(
        [diaria, "stats"],
        env=vault.env,
        check=True,
        stdout=subprocess.PIPE,
        encoding="utf-8",
    ).stdout

    assert _row_count(stats_output) == 9


def test_stats_empty_vault(diaria: Path, vault: Vault):
    """With no entries, `stats` still exits cleanly and prints nothing."""
    result = subprocess.run(
        [diaria, "stats"],
        env=vault.env,
        check=True,
        stdout=subprocess.PIPE,
        encoding="utf-8",
    )
    assert result.stdout.strip() == ""
