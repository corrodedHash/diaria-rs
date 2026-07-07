import subprocess
from pathlib import Path
from .helper import diaria

def test_find_executable(diaria: Path):
    subprocess.run([str(diaria), "--help"], check=True)
    subprocess.run([str(diaria), "--version"], check=True)
