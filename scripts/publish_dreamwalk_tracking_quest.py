"""Produce SQL for native Dreamwalk's tracking quest; never mutate the source database."""
import argparse
import sqlite3
from pathlib import Path


def render_sql(conn):
    return "-- Dreamwalk tracking quest publication pending.\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.output.resolve() == args.source.resolve():
        parser.error("SQL output must not overwrite the source database")
    with sqlite3.connect(f"{args.source.resolve().as_uri()}?mode=ro", uri=True) as conn:
        sql = render_sql(conn)
    args.output.write_text(sql)


if __name__ == "__main__":
    main()
