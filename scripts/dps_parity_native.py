#!/usr/bin/env python3
"""Run opt-in native parity test executables against one read-only DB snapshot.

Requires Linux bubblewrap. Both startup WORLD_DB_PATH and DPS_PARITY_WORLD_DB
resolve to the prepared file inside the child mount namespace; host DBs do not
change. No unsandboxed execution path is provided.
"""
import os
from pathlib import Path
import sys
from typing import Sequence


def build_native_command(
    startup_database: Path, prepared_database: Path, command: Sequence[str]
) -> list[str]:
    """Create the explicit source view; paths have already been resolved."""
    return [
        "bwrap",
        "--unshare-user",
        "--die-with-parent",
        "--bind",
        "/",
        "/",
        "--ro-bind",
        str(prepared_database),
        str(startup_database),
        "--",
        *command,
    ]


def main(arguments: Sequence[str]) -> None:
    if not arguments:
        raise ValueError("provide a compiled native parity test executable")
    root = Path(__file__).resolve().parents[1]
    executable = Path(arguments[0]).resolve(strict=True)
    if (
        not executable.name.startswith("game_server-")
        or not executable.is_relative_to(root / "target")
        or not os.access(executable, os.X_OK)
    ):
        raise ValueError(
            "only this checkout's executable game_server-* test binaries are allowed"
        )
    prepared_value = os.environ.get("DPS_PARITY_WORLD_DB")
    if prepared_value is None:
        raise ValueError("DPS_PARITY_WORLD_DB must name the frozen native snapshot")
    prepared = Path(prepared_value).resolve(strict=True)
    startup = (root / "data/world.db").resolve(strict=True)
    command = build_native_command(startup, prepared, [str(executable), *arguments[1:]])
    os.execvp(command[0], command)


if __name__ == "__main__":
    try:
        main(sys.argv[1:])
    except (OSError, ValueError) as error:
        raise SystemExit(f"native parity source view: {error}") from error
