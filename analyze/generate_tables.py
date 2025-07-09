import subprocess
import os
from pathlib import Path

THIS_DIR = os.path.dirname(os.path.abspath(__file__))

# Top level of pbc-compiler
TOP_DIR = Path(THIS_DIR, '..')

def generate_measurement_tables():
    command = [f"{TOP_DIR}/scripts/generate_measurement_tables.sh"]
    subprocess.run(command, stdout=None, shell=False, check=True)
