"""Run the pinned wow-formats-dump CLI and parse its one-line JSON projection.

The CLI decodes WoW binaries with the same wow-formats crates the server uses;
callers keep their own selection, validation and error policy.
"""

import json
import os
import subprocess
from pathlib import Path

REPO = "https://github.com/world-of-osso/wow-formats.git"
REV = "8ef0d78313b9ffc468f6fc3e62be707384ab3e07"
ROOT = (
    Path(os.environ.get("XDG_CACHE_HOME") or Path.home() / ".cache")
    / "wow-formats-dump"
    / REV
)
BINARY = ROOT / "bin" / "wow-formats-dump"
INSTALL = (
    f"cargo install --locked --git {REPO} --rev {REV} wow-formats-dump --root {ROOT}"
)


def dump(command, path):
    """Return the JSON projection; missing CLI or decode failure raise ValueError."""
    if not BINARY.is_file():
        raise ValueError(f"wow-formats-dump {REV[:9]} is not installed; run: {INSTALL}")
    result = subprocess.run(
        [str(BINARY), command, str(path)], capture_output=True, text=True, check=False
    )
    if result.returncode != 0:
        raise ValueError(
            result.stderr.strip() or f"wow-formats-dump exit {result.returncode}"
        )
    # Non-finite floats arrive as NaN/Infinity tokens, which json.loads accepts.
    return json.loads(result.stdout)
