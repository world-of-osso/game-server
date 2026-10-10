#!/usr/bin/env python3
"""Export local Toy.db2 using the engine's maintained WDC5 decoder.

Layout 821B5D20: WoWDBDefs Toy.dbd, Retail 12.1.0.69933.
No network/CASC fetching: supply the locally extracted FDID 988200.
"""
import argparse
import importlib.util
import sys
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("db2", type=Path)
    parser.add_argument("csv", type=Path)
    parser.add_argument("--exporter", type=Path, required=True)
    args = parser.parse_args()
    spec = importlib.util.spec_from_file_location("export_db2_csv", args.exporter)
    exporter = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(exporter)
    exporter.TABLES["Toy"] = (
        0x821B5D20,
        [("ID", "id"), ("SourceText_lang", ("string", 0)),
         ("ItemID", 2), ("Flags", 3), ("SourceTypeEnum", ("i8", 4))],
    )
    exporter.INLINE_ID_FIELD["Toy"] = 1
    sys.argv = [str(args.exporter), "Toy", str(args.db2), str(args.csv)]
    exporter.main()


if __name__ == "__main__":
    main()
