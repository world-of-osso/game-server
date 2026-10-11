#!/usr/bin/env python3
"""Export build-pinned Lock.db2 for GO use validation; never writes world.db.

Local CASC FDID1343608; WoWDBDefs Lock.dbd layout DA418152 (12.1.0.69933):
noninline ID, Flags32, Index32[8], Skill16[8], Type8[8], Action8[8].
"""
import argparse
import json
from pathlib import Path

from db2_casc import decode

LAYOUT = 0xDA418152


def decode_locks(data):
    header, rows = decode(data)
    if header["layout_hash"] != LAYOUT or header["record_size"] != 68:
        raise ValueError("Lock.db2 requires layout DA418152, 68-byte records")
    locks = {}
    for lock_id, row in rows.items():
        if len(row) != 6:
            raise ValueError(f"Lock {lock_id}: expected ID and five array/scalar fields")
        _, _, indices, skills, types, actions = row
        locks[str(lock_id)] = [
            {"kind": (types >> (slot * 8)) & 0xFF,
             "index": (indices >> (slot * 32)) & 0xFFFFFFFF,
             "skill": (skills >> (slot * 16)) & 0xFFFF,
             "action": (actions >> (slot * 8)) & 0xFF}
            for slot in range(8) if (types >> (slot * 8)) & 0xFF
        ]
    return {"layout_hash": LAYOUT, "locks": locks}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--db2", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    content = decode_locks(args.db2.read_bytes())
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(content, sort_keys=True) + "\n")
    print(f"{len(content['locks'])} authored locks → {args.output}")


if __name__ == "__main__":
    main()
