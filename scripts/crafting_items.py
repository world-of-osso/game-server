#!/usr/bin/env python3
"""Add missing crafting-referenced Retail items without changing existing content.

Requires imported crafting_schema.sql reference tables and Retail ItemSparse/Item
sources. Missing source rows are reported, never fabricated. --db and --report
are explicit; only the supplied database is updated, in one transaction.
"""

import argparse
import json
from pathlib import Path
import sqlite3

from content_policy import ITEM_COLUMNS
from economy_crafting import crafting_item_ids


def project_crafting_items(db):
    """Add genuine missing rows atomically; preserve a caller-owned transaction.

    Return sorted added/existing IDs and missing_item/missing_item_sparse IDs.
    No commits of caller changes, row_factory changes, or recipe eligibility gate.
    """
    references = crafting_item_ids(db)
    existing = references & {
        row[0] for row in db.execute("SELECT ID FROM content_item")
    }
    result = {
        "referenced": len(references),
        "existing": sorted(existing),
        "added": [],
        "missing_item": [],
        "missing_item_sparse": [],
    }
    projection = f"SELECT {ITEM_COLUMNS}, NULL AS SourceEntry FROM item_sparse s JOIN item i ON i.ID=s.ID WHERE s.ID=?"
    cursor = db.execute(projection, (0,))
    columns = ", ".join(f'"{column[0]}"' for column in cursor.description)
    insert = f"INSERT INTO content_item ({columns}) {projection}"
    db.execute("SAVEPOINT crafting_item_projection")
    try:
        for item_id in sorted(references - existing):
            has_item = (
                db.execute("SELECT 1 FROM item WHERE ID=?", (item_id,)).fetchone()
                is not None
            )
            has_sparse = (
                db.execute(
                    "SELECT 1 FROM item_sparse WHERE ID=?", (item_id,)
                ).fetchone()
                is not None
            )
            if not has_item:
                result["missing_item"].append(item_id)
            if not has_sparse:
                result["missing_item_sparse"].append(item_id)
            if has_item and has_sparse:
                db.execute(insert, (item_id,))
                result["added"].append(item_id)
    except BaseException:
        db.execute("ROLLBACK TO crafting_item_projection")
        db.execute("RELEASE crafting_item_projection")
        raise
    db.execute("RELEASE crafting_item_projection")
    return result


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--db", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args(argv)
    path = args.db.resolve()
    if not path.is_file():
        parser.error(f"Database does not exist: {path}")
    if args.report.resolve() == path:
        parser.error("Report path must differ from database path")
    # mode=rw cannot silently create an empty database on a mistyped path.
    db = sqlite3.connect(f"{path.as_uri()}?mode=rw", uri=True)
    try:
        with db:
            db.execute("BEGIN IMMEDIATE")
            result = project_crafting_items(db)
    finally:
        db.close()
    result["db"] = str(path)
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(result, sort_keys=True))


if __name__ == "__main__":
    main()
