#!/usr/bin/env python3
"""Import the local Toy.csv catalog into world.db. Preserve unresolved rows."""
import argparse
import csv
import json
import sqlite3
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("csv", type=Path)
    parser.add_argument("database", type=Path)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument(
        "--item-id", type=int, action="append",
        help="Add or refresh only this source item; repeat for a selected batch",
    )
    args = parser.parse_args()
    with args.csv.open() as handle:
        rows = list(csv.DictReader(handle))
    if args.item_id:
        requested = set(args.item_id)
        rows = [row for row in rows if int(row["ItemID"]) in requested]
        missing = sorted(requested - {int(row["ItemID"]) for row in rows})
        if missing:
            parser.error(f"requested toy items absent from source: {missing}")
    with sqlite3.connect(args.database) as conn:
        conn.execute("CREATE TABLE IF NOT EXISTS toy (ID INTEGER PRIMARY KEY, ItemID INTEGER UNIQUE NOT NULL, Flags INTEGER NOT NULL, SourceTypeEnum INTEGER NOT NULL, SourceText_lang TEXT NOT NULL)")
        if args.item_id:
            conn.executemany("DELETE FROM toy WHERE ItemID=?", [(item,) for item in args.item_id])
        else:
            conn.execute("DELETE FROM toy")
        conn.executemany("INSERT INTO toy VALUES (?,?,?,?,?)", [
            (int(row['ID']), int(row['ItemID']), int(row['Flags']),
             int(row['SourceTypeEnum']), row['SourceText_lang']) for row in rows
        ])
        unresolved = []
        counts = {'toys': len(rows), 'item': 0, 'item_sparse': 0, 'on_use': 0, 'spell': 0}
        for row in rows:
            item = int(row['ItemID'])
            missing = []
            for table in ('item', 'item_sparse'):
                if conn.execute(f'SELECT 1 FROM {table} WHERE ID=?', (item,)).fetchone():
                    counts[table] += 1
                else:
                    missing.append(table)
            effects = conn.execute('SELECT e.SpellID FROM item_x_item_effect x JOIN item_effect e ON e.ID=x.ItemEffectID WHERE x.ItemID=? AND e.TriggerType=0 AND e.SpellID>0 ORDER BY e.LegacySlotIndex,e.ID', (item,)).fetchall()
            if effects:
                counts['on_use'] += 1
                if conn.execute('SELECT 1 FROM spell_effect WHERE SpellID=? AND DifficultyID=0', effects[0]).fetchone():
                    counts['spell'] += 1
                else:
                    missing.append('spell_effect')
            else:
                missing.append('on_use')
            if missing:
                unresolved.append({'toy_id': int(row['ID']), 'item_id': item, 'missing': missing})
        report = {'counts': counts, 'unresolved': unresolved}
        args.report.write_text(json.dumps(report, indent=2) + '\n')
        print(json.dumps(counts), 'unresolved:', len(unresolved))


if __name__ == '__main__':
    main()
