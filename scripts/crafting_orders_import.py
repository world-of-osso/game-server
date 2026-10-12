#!/usr/bin/env python3
"""Import local CraftingOrder CSV into a NEW SQLite copy, never the source DB."""
import argparse
from contextlib import closing
import csv
import json
from pathlib import Path
import sqlite3

from export_db2_csv import COLUMNS

FIXED_OUTPUT_SQL = '''SELECT o.ID,a.Spell,a.SkillLine,e.EffectItemType,
    MAX(1,CAST(ROUND(e.EffectBasePointsF) AS INTEGER))
    FROM crafting_order o JOIN skill_line_ability a ON a.ID=o.SkillLineAbilityID
    JOIN spell_effect e ON e.SpellID=a.Spell
    WHERE e.Effect=24 AND e.DifficultyID=0 AND e.EffectItemType>0'''


def import_copy(source, output, csv_path):
    source, output = Path(source).resolve(), Path(output).resolve()
    if source == output:
        raise ValueError('output must be a new copy, not the source database')
    if output.exists():
        raise FileExistsError(output)
    with Path(csv_path).open(newline='') as stream:
        reader = csv.reader(stream)
        if tuple(next(reader, ())) != COLUMNS:
            raise ValueError('unexpected CraftingOrder CSV header')
        rows = [tuple(int(value) for value in row) for row in reader]
    if any(len(row) != 4 or row[0] <= 0 for row in rows):
        raise ValueError('invalid CraftingOrder row')
    if len({row[0] for row in rows}) != len(rows):
        raise ValueError('duplicate CraftingOrder identity')
    # Reserve the destination before opening SQLite, so existing DBs are never overwritten.
    output.touch(exist_ok=False)
    try:
        with closing(sqlite3.connect(f'{source.as_uri()}?mode=ro', uri=True)) as src:
            with closing(sqlite3.connect(output)) as dst:
                src.backup(dst)
                with dst:
                    dst.execute('DROP TABLE IF EXISTS crafting_order')
                    dst.execute('CREATE TABLE crafting_order (ID INTEGER PRIMARY KEY, SkillLineAbilityID INTEGER NOT NULL, CraftingOrderHouseID INTEGER NOT NULL, Field_10_0_2_46479_002 INTEGER NOT NULL)')
                    dst.executemany('INSERT INTO crafting_order VALUES (?,?,?,?)', rows)
                fixed = dst.execute(FIXED_OUTPUT_SQL).fetchall()
                return {'rows': len(rows), 'fixed_output_rows': len(fixed), 'output': str(output)}
    except Exception:
        output.unlink()
        raise


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-db', type=Path, required=True)
    parser.add_argument('--output-db', type=Path, required=True)
    parser.add_argument('--csv', type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(import_copy(args.source_db, args.output_db, args.csv), indent=2))


if __name__ == '__main__':
    main()
