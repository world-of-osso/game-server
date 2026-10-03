#!/usr/bin/env python3
"""Watch server sources and restart the native build/run loop on changes."""

import os
from pathlib import Path
import shlex
import sys


if __name__ == "__main__":
    loop = Path(__file__).resolve().parent / "scripts/dev-loop.py"
    command = shlex.join([sys.executable, str(loop), *sys.argv[1:]])
    os.execvp(
        "cargo", ["cargo", "watch", "-w", "crates", "-w", "Cargo.toml", "-s", command]
    )
