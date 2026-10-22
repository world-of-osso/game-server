#!/usr/bin/env python3
"""Frozen database view for opt-in native parity test executables."""
from pathlib import Path
from typing import Sequence


def build_native_command(
    startup_database: Path, prepared_database: Path, command: Sequence[str]
) -> list[str]:
    """Existing direct-execution boundary, before source-view qualification."""
    return list(command)
