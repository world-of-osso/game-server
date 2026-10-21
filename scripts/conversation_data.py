#!/usr/bin/env python3
"""Local TDB/70448 conversation source ingress; canonical publication is lead-owned."""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_SOURCE = ROOT / 'data/reference/tdb1210/TDB_full_world_1210.26091_2026_09_09.sql'
DEFAULT_CSV = ROOT / 'data/db2/12.1.5.70448'


def read_source_tables(source=DEFAULT_SOURCE, csv_dir=DEFAULT_CSV):
    return {}
