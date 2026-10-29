#!/usr/bin/env python3
"""Extract local CASC model dependencies and bake the complete LOS eye table."""

import argparse
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--world-db", type=Path, required=True)
    parser.add_argument("--engine", type=Path, required=True)
    parser.add_argument("--baker", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    subprocess.run([sys.executable, ROOT / "scripts/los-extract-models.py", "--world-db", args.world_db,
                    "--engine", args.engine], check=True)
    subprocess.run([args.baker, "--eyes-only", "--data", args.engine / "data", "--world-db", args.world_db,
                    "--out", args.out], check=True)
    subprocess.run([sys.executable, ROOT / "scripts/check_los_eyes.py", "--world-db", args.world_db,
                    "--table", args.out / "eye_heights.tsv"], check=True)


if __name__ == "__main__":
    main()
