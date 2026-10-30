#!/usr/bin/env python3
"""Load the TDB1210 guild reward tables into a prepared world.db copy.

Cuts the `guild_rewards` and `guild_rewards_req_achievements` INSERTs out of the full
TDB world dump into per-table excerpts (the files import-azerothcore.sh reads), then
loads them as tdb_guild_rewards* with guild_rewards_schema.sql.

  import_guild_rewards.py --db COPY --dump TDB_full_world_1210.26091_2026_09_09.sql
"""
import argparse
from contextlib import closing
from pathlib import Path
import sqlite3

from mysql_dump import load_table

TABLES = ("guild_rewards", "guild_rewards_req_achievements")
SCHEMA = Path(__file__).resolve().parent / "guild_rewards_schema.sql"


def excerpt(dump_text, table):
    prefix = f"INSERT INTO `{table}` "
    lines = [line for line in dump_text.splitlines() if line.startswith(prefix)]
    if not lines:
        raise ValueError(f"dump has no {table} rows")
    return "\n".join(lines) + "\n"


def write_excerpts(dump_path, excerpt_dir):
    text = dump_path.read_text(encoding="utf-8")
    paths = {}
    for table in TABLES:
        paths[table] = excerpt_dir / f"{table}.sql"
        paths[table].write_text(excerpt(text, table), encoding="utf-8")
    return paths


def import_guild_rewards(conn, excerpts):
    """Replace both tables in the caller's transaction (`with conn`)."""
    conn.executescript("BEGIN;\n" + SCHEMA.read_text())
    return {table: load_table(conn, table, excerpts[table], "tdb_") for table in TABLES}


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--db", required=True, type=Path)
    parser.add_argument("--dump", required=True, type=Path)
    parser.add_argument("--excerpt-dir", type=Path, help="defaults to the dump's directory")
    args = parser.parse_args()
    canonical = Path(__file__).resolve().parents[1] / "data/world.db"
    if not args.db.is_file() or args.db.name == "world.db" or args.db.samefile(canonical):
        parser.error("provide an existing prepared copy, not world.db")
    if args.db.is_symlink() or args.db.stat().st_nlink != 1:
        parser.error("copy must be an independent regular file")
    excerpts = write_excerpts(args.dump, args.excerpt_dir or args.dump.parent)
    with closing(sqlite3.connect(args.db)) as conn, conn:
        counts = import_guild_rewards(conn, excerpts)
    for table, count in counts.items():
        print(f"tdb_{table}: {count}")


if __name__ == "__main__":
    main()
