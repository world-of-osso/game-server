#!/usr/bin/env python3
"""Lead-owned runtime Kotmogu orb import, after content_policy.py; local TDB templates only."""
import argparse
import json
import sqlite3
from contextlib import closing
from pathlib import Path

from battleground_summons import import_kotmogu_orbs


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--db', type=Path, required=True)
    args = parser.parse_args()
    with closing(sqlite3.connect(args.db.resolve().as_uri() + '?mode=rw', uri=True)) as conn:
        with conn:
            conn.execute('BEGIN')
            count = import_kotmogu_orbs(conn)
    print(json.dumps({'map': 998, 'scripted_orbs': count, 'db': str(args.db)}))


if __name__ == '__main__':
    main()
