#!/usr/bin/env python3
"""Publish local Item/ItemSparse Mythic Keystone180653 into an independent world copy."""
import argparse
import sqlite3
from contextlib import closing
from pathlib import Path
from import_mythic_plus import validate_destination


def import_item(destination):
    validate_destination(Path(destination))
    with closing(sqlite3.connect(destination)) as conn, conn:
        conn.row_factory = sqlite3.Row
        sparse = conn.execute("SELECT * FROM item_sparse WHERE ID=180653").fetchone()
        item = conn.execute("SELECT * FROM item WHERE ID=180653").fetchone()
        if sparse is None or item is None:
            raise ValueError("Local Item/ItemSparse180653 is missing")
        values = dict(sparse)
        values.update(dict(item))
        values["Name"] = sparse["Display_lang"]
        values["Quality"] = sparse["OverallQualityID"]
        columns = [row[1] for row in conn.execute("PRAGMA table_info(content_item)") if row[1] in values]
        if "ID" not in columns or "Name" not in columns:
            raise ValueError("content_item schema is missing ID/Name")
        names = ",".join(f'"{column}"' for column in columns)
        placeholders = ",".join("?" for _ in columns)
        conn.execute(f"INSERT OR REPLACE INTO content_item ({names}) VALUES ({placeholders})", [values[column] for column in columns])
        return True


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--db", required=True, type=Path)
    args = parser.parse_args()
    import_item(args.db)
