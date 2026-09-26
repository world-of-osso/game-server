"""Helpers shared by content_policy.py and retail_content.py."""

import csv
from collections import Counter


def clone_table(conn, source, target):
    """Create empty `target` with `source`'s DDL (keeps column types and primary key)."""
    ddl = conn.execute("SELECT sql FROM sqlite_master WHERE type = 'table' AND name = ?", (source,)).fetchone()[0]
    conn.execute(f"DROP TABLE IF EXISTS {target}")
    conn.execute(ddl.replace(f"CREATE TABLE {source}", f"CREATE TABLE {target}", 1))


class Report:
    def __init__(self):
        self.rows = []
        self.counts = Counter()

    def drop(self, table, key, source_id, reason):
        self.rows.append((table, key, source_id, reason))
        self.counts[table] += 1

    def write(self, path):
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open("w", newline="", encoding="utf-8") as handle:
            writer = csv.writer(handle)
            writer.writerow(["table", "key", "source_id", "reason"])
            writer.writerows(self.rows)
