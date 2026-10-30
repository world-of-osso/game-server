#!/usr/bin/env python3
"""Project authored TDB creature static flags into an existing content database copy."""
import argparse
import sqlite3

from retail_content import resolve_difficulty_rows


FLAGS = tuple(f"StaticFlags{i}" for i in range(1, 9))


def project_static_flags(conn):
    for table in ("content_creature_template", "content_creature_template_difficulty"):
        existing = {row[1] for row in conn.execute(f"PRAGMA table_info({table})")}
        if existing.intersection(FLAGS):
            raise ValueError(f"{table} already has static flag columns")
        for column in FLAGS:
            conn.execute(f"ALTER TABLE {table} ADD COLUMN {column} INTEGER NOT NULL DEFAULT 0")

    pairs = conn.execute("SELECT entry, DifficultyID FROM content_creature_template_difficulty").fetchall()
    resolved = resolve_difficulty_rows(conn, pairs)
    missing = set(pairs) - resolved.keys()
    if missing:
        raise ValueError(f"missing source difficulty rows: {sorted(missing)[:10]}")
    conn.execute("CREATE TEMP TABLE static_flag_sources (entry INTEGER, difficulty INTEGER, source INTEGER, PRIMARY KEY(entry,difficulty))")
    conn.executemany("INSERT INTO static_flag_sources VALUES (?,?,?)",
                     [(entry, difficulty, source) for (entry, difficulty), source in sorted(resolved.items())])
    columns = ",".join(FLAGS)
    source_columns = ",".join(f"d.{flag}" for flag in FLAGS)
    conn.execute(f"""UPDATE content_creature_template_difficulty AS c SET ({columns}) = (
        SELECT {source_columns} FROM temp.static_flag_sources s
        JOIN tdb_creature_template_difficulty d ON d.Entry=s.entry AND d.DifficultyID=s.source
        WHERE s.entry=c.entry AND s.difficulty=c.DifficultyID)""")
    conn.execute("""CREATE TEMP TABLE static_flag_base AS SELECT entry, source FROM temp.static_flag_sources s
        WHERE difficulty = (SELECT MIN(difficulty) FROM temp.static_flag_sources WHERE entry=s.entry)""")
    conn.execute("CREATE UNIQUE INDEX temp.static_flag_base_entry ON static_flag_base(entry)")
    conn.execute(f"""UPDATE content_creature_template AS c SET ({columns}) = (
        SELECT {source_columns} FROM tdb_creature_template_difficulty d
        WHERE d.Entry=c.entry AND d.DifficultyID=COALESCE(
            (SELECT source FROM temp.static_flag_base b WHERE b.entry=c.entry), 0))
        WHERE EXISTS (SELECT 1 FROM tdb_creature_template_difficulty d WHERE d.Entry=c.entry AND
            d.DifficultyID=COALESCE((SELECT source FROM temp.static_flag_base b WHERE b.entry=c.entry), 0))""")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("database_copy", help="writable copy of the content database")
    args = parser.parse_args()
    with sqlite3.connect(args.database_copy) as conn:
        project_static_flags(conn)


if __name__ == "__main__":
    main()
