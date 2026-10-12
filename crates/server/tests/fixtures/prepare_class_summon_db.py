"""Read-only native snapshot plus production class-summon import for process fixtures."""
import argparse
from contextlib import closing
from pathlib import Path
import sqlite3
import sys

ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT / "scripts"))
import add_class_summon_templates


def prepare(source, destination):
    if destination.exists() or destination.name == "world.db":
        raise ValueError("class fixture requires a fresh owned database copy")
    with closing(sqlite3.connect(f"file:{source.resolve()}?mode=ro", uri=True)) as native:
        with closing(sqlite3.connect(destination)) as copied, copied:
            native.backup(copied)
            added = add_class_summon_templates.add_class_summon_templates(copied)
    return added


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("destination", type=Path)
    args = parser.parse_args()
    added = prepare(args.source, args.destination)
    print(f"class fixture prepared: {len(added)} authentic missing templates, no world spawns")


if __name__ == "__main__":
    main()
