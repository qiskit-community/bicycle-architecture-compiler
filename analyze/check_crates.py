import subprocess
import os
from pathlib import Path

THIS_DIR = os.path.dirname(os.path.abspath(__file__))

# Top level directory of bicycle-architecture-compiler
TOP_DIR = Path(THIS_DIR, '..')

def compile_crates():
    command = [
        "cargo",
        "build",
        "-r",
        "--manifest-path",
        f"{TOP_DIR}/Cargo.toml"
    ]
    subprocess.run(command, stdout=None, shell=False, check=True)
