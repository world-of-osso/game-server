#!/usr/bin/env python3
"""Load the INSERT rows of AzerothCore mysqldump files into existing SQLite tables.

  mysql_dump.py --db PATH [--schema DDL.sql] [--prefix P] TABLE=DUMP.sql [TABLE=DUMP.sql ...]

--schema runs DDL (e.g. world_schema.sql, which drops and recreates the dump
tables) in the same transaction as the load, so re-importing into a populated
world.db replaces only those tables and a failed load leaves them as they were.

Each dump's `INSERT INTO `table` [(cols)] VALUES (...),(...);` statements are
tokenized with MySQL string escapes (\\' \\" \\\\ \\n \\r \\t \\0 \\Z) decoded, and
inserted through parameters, so text reaches SQLite verbatim. Unsigned 64-bit
integers above 2^63-1 are stored as their two's-complement (signed) bit pattern. --prefix loads each
dump table into `<P><table>` (TrinityCore `trainer` → `tdb_trainer`), so dumps
from another source never replace the AzerothCore tables of the same name. All tables load in
one transaction; any unparseable token fails the whole load.
"""

import argparse
import re
import sqlite3
import sys
from pathlib import Path

INSERT_RE = re.compile(r"^INSERT INTO `(\w+)`\s*(?:\(([^)]*)\))?\s*VALUES\s*", re.M)
TOKEN_RE = re.compile(
    r"\s*(?:'((?:[^'\\]|\\.|'')*)'|(NULL)|(-?\d+\.\d*(?:[eE][-+]?\d+)?|-?\d+[eE][-+]?\d+)|(-?\d+)|([(),;]))",
    re.S,
)
ESCAPE_RE = re.compile(r"\\(.)|''", re.S)
ESCAPES = {"0": "\0", "b": "\b", "n": "\n", "r": "\r", "t": "\t", "Z": "\x1a"}


def unescape(body):
    def replace(match):
        char = match.group(1)
        if char is None:
            return "'"
        if char in "%_":  # MySQL keeps the backslash for these outside LIKE.
            return "\\" + char
        return ESCAPES.get(char, char)

    return ESCAPE_RE.sub(replace, body)


def parse_values(text, pos, source):
    """Yield row tuples from `(...),(...);` starting at pos; stop after the terminating `;`."""
    row = None
    while True:
        match = TOKEN_RE.match(text, pos)
        if not match:
            raise ValueError(f"{source}: unparseable value at offset {pos}: {text[pos:pos + 60]!r}")
        pos = match.end()
        string, null, real, integer, punct = match.groups()
        if punct == "(":
            row = []
        elif punct == ")":
            yield tuple(row)
            row = None
        elif punct == ";":
            return
        elif punct == ",":
            continue
        elif row is None:
            raise ValueError(f"{source}: value outside a row at offset {match.start()}")
        elif string is not None:
            row.append(unescape(string))
        elif null:
            row.append(None)
        elif real is not None:
            row.append(float(real))
        else:
            value = int(integer)
            # MySQL BIGINT UNSIGNED masks (TrinityCore AllowableRaces) exceed SQLite's int64:
            # store their two's-complement bit pattern.
            row.append(value - (1 << 64) if value >= 1 << 63 else value)


def dump_rows(text, table, source):
    """Yield (columns or None, row) for every INSERT into `table` in a dump."""
    for match in INSERT_RE.finditer(text):
        if match.group(1) != table:
            raise ValueError(f"{source}: INSERT into {match.group(1)}, expected {table}")
        columns = [c.strip().strip("`") for c in match.group(2).split(",")] if match.group(2) else None
        for row in parse_values(text, match.end(), source):
            yield columns, row


def load_table(conn, table, dump_path, prefix=""):
    text = Path(dump_path).read_text(encoding="utf-8")
    count = 0
    for columns, row in dump_rows(text, table, dump_path):
        column_sql = f" ({','.join(columns)})" if columns else ""
        conn.execute(f"INSERT INTO {prefix}{table}{column_sql} VALUES ({','.join('?' * len(row))})", row)
        count += 1
    return count


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--db", type=Path, required=True)
    parser.add_argument("--schema", type=Path)
    parser.add_argument("--prefix", default="")
    parser.add_argument("pairs", nargs="+", metavar="TABLE=DUMP")
    args = parser.parse_args()
    conn = sqlite3.connect(args.db, isolation_level=None)
    try:
        conn.executescript("BEGIN;\n" + (args.schema.read_text() if args.schema else ""))
        for pair in args.pairs:
            table, dump_path = pair.split("=", 1)
            print(f"  {args.prefix}{table}: {load_table(conn, table, dump_path, args.prefix)}")
        conn.execute("COMMIT")
    except BaseException:
        if conn.in_transaction:
            conn.execute("ROLLBACK")
        raise
    finally:
        conn.close()


if __name__ == "__main__":
    sys.exit(main())
