#!/usr/bin/env python3
"""Copy-only site catalog: local Retail races plus source-isolated Skyborne races."""
import argparse
import csv
import sqlite3
from pathlib import Path

SKYBORNE_RACE_IDS = {95, 96}

def read_races(path, skyborne):
    with Path(path).open(newline="", encoding="utf-8-sig") as file:
        rows = []
        for row in csv.DictReader(file):
            race_id = int(row["ID"])
            if (race_id in SKYBORNE_RACE_IDS) == skyborne:
                rows.append((race_id, row["Name_lang"], int(row["FactionID"]), int(row["CreatureType"])))
        return rows

def prepare(source, retail_races, forever_races, output):
    source = Path(source).resolve(strict=True)
    output = Path(output)
    rows = read_races(retail_races, False) + read_races(forever_races, True)
    if len({row[0] for row in rows}) != len(rows):
        raise ValueError("race catalogs contain duplicate selected IDs")
    output.touch(exist_ok=False)
    try:
        with sqlite3.connect(source.as_uri() + "?mode=ro", uri=True) as original:
            with sqlite3.connect(output) as copy:
                original.backup(copy)
                copy.execute("DROP TABLE IF EXISTS chr_races")
                copy.execute("CREATE TABLE chr_races(ID INTEGER PRIMARY KEY, Name_lang TEXT NOT NULL, FactionID INTEGER NOT NULL, CreatureType INTEGER NOT NULL)")
                copy.executemany("INSERT INTO chr_races VALUES(?,?,?,?)", rows)
    except Exception:
        output.unlink()
        raise

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", required=True, type=Path)
    parser.add_argument("--retail-races", required=True, type=Path)
    parser.add_argument("--forever-races", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    prepare(args.source, args.retail_races, args.forever_races, args.output)
