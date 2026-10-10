#!/usr/bin/env python3
"""Import locally extracted Retail ItemDisenchantLoot (FDID1134087, layout5845A4AA).

Uses the maintained engine WDC5 decoder. No network or legacy eligibility pointers.
"""
import argparse
import importlib.util
import sqlite3
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("db2", type=Path)
    parser.add_argument("world", type=Path)
    parser.add_argument("--exporter", type=Path, required=True)
    args = parser.parse_args()
    spec = importlib.util.spec_from_file_location("export_db2_csv", args.exporter)
    exporter = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(exporter)
    rows, dropped, _, _ = exporter.read_wdc5(args.db2.read_bytes(), 0x5845A4AA)
    if dropped:
        raise ValueError(f"ItemDisenchantLoot: {dropped} encrypted records unavailable")
    records = []
    for row_id, (values, parent, _) in sorted(rows.items()):
        subclass, quality, minimum, maximum, skill, expansion = values
        subclass = subclass - 256 if subclass >= 128 else subclass
        expansion = expansion - 256 if expansion >= 128 else expansion
        if parent is None or minimum > maximum:
            raise ValueError(f"invalid ItemDisenchantLoot row {row_id}")
        records.append((row_id, subclass, quality, minimum, maximum, skill, expansion, parent))
    with sqlite3.connect(args.world) as conn:
        conn.execute("CREATE TABLE IF NOT EXISTS item_disenchant_loot (ID INTEGER PRIMARY KEY, Subclass INTEGER NOT NULL, Quality INTEGER NOT NULL, MinLevel INTEGER NOT NULL, MaxLevel INTEGER NOT NULL, SkillRequired INTEGER NOT NULL, ExpansionID INTEGER NOT NULL, Class INTEGER NOT NULL)")
        conn.execute("DELETE FROM item_disenchant_loot")
        conn.executemany("INSERT INTO item_disenchant_loot VALUES (?, ?, ?, ?, ?, ?, ?, ?)", records)
    print(f"ItemDisenchantLoot: {len(records)} authentic rows imported")


if __name__ == "__main__":
    main()
