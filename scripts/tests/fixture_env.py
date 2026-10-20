"""Real-data fixture paths named by environment variables.

Tests that need an unset variable skip with the variable named in the reason, so a
plain unittest run stays clean; scripts given the same paths still fail explicitly.
"""

import os
import unittest
from pathlib import Path


def env_path(name):
    """The fixture path in `name`, or None when the variable is unset or empty."""
    value = os.environ.get(name)
    return Path(value) if value else None


def requires_env(name):
    """Skip a test or class unless `name` points at its local real-data fixture."""
    return unittest.skipUnless(
        os.environ.get(name),
        f"{name} is not set (local real-data fixture; see docs/db2-casc-sparse.md)",
    )
