#!/usr/bin/env python3
"""Pin mount-only ItemExtendedCost rows from local CASC, without writing world.db."""
import argparse
import csv
import hashlib
import importlib.util
import json
import sqlite3
from pathlib import Path

MOUNT_ITEMS = """
SELECT DISTINCT x.ItemID FROM item_x_item_effect x
JOIN item_effect e ON e.ID=x.ItemEffectID AND e.TriggerType=6
JOIN mount m ON m.SourceSpellID=e.SpellID
"""


def export(world, db2, reader, output):
    spec = importlib.util.spec_from_file_location("db2_export", reader)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    data = db2.read_bytes()
    rows, dropped, _, _ = module.read_wdc5(data, 0x22331DAB)
    if dropped:
        raise ValueError(f"ItemExtendedCost contains {dropped} unreadable records")
    connection = sqlite3.connect(f"{world.resolve().as_uri()}?mode=ro", uri=True)
    offers = connection.execute(f"""
        SELECT entry, slot, item, ExtendedCost FROM tdb_npc_vendor
        WHERE type=1 AND ExtendedCost<>0 AND item IN ({MOUNT_ITEMS})
        ORDER BY entry, slot, item, ExtendedCost
    """).fetchall()
    ids = sorted({offer[3] for offer in offers})
    unavailable = set()
    unavailable_items = set()
    result = []
    for cost_id in ids:
        values, _, _ = rows[cost_id]
        if any(values[1:7]):
            raise ValueError(f"cost {cost_id}: unsupported constraints {values[1:7]}")
        money = values[0]
        if not 0 <= money <= 0xFFFFFFFF:
            raise ValueError(f"cost {cost_id}: money exceeds vendor price field")
        row = [cost_id, money]
        for kind, id_index, count_index in (("item", 7, 8), ("currency", 9, 10)):
            for identifier, amount in zip(values[id_index], values[count_index], strict=True):
                if bool(identifier) != bool(amount):
                    raise ValueError(f"cost {cost_id}: mismatched {kind} ID/count")
                if kind == "currency" and identifier:
                    metadata = connection.execute(
                        "SELECT Flags_0 FROM currency_types WHERE ID=?", (identifier,)
                    ).fetchone()
                    if metadata is None:
                        unavailable.add(identifier)
                    elif metadata[0] & 0x8 and amount % 100:
                        raise ValueError(f"cost {cost_id}: fractional currency {identifier}: {amount}/100")
                if kind == "item" and identifier:
                    metadata = connection.execute(
                        "SELECT 1 FROM content_item c JOIN item i ON i.ID=c.ID WHERE c.ID=?",
                        (identifier,),
                    ).fetchone()
                    if metadata is None:
                        unavailable_items.add(identifier)
                row.extend((identifier, amount))
        result.append(row)
    with output.open("w", newline="") as stream:
        writer = csv.writer(stream, lineterminator="\n")
        writer.writerow(["extended_cost", "money"] + [
            field for kind in ("item", "currency") for index in range(5)
            for field in (f"{kind}_{index}", f"{kind}_count_{index}")
        ])
        writer.writerows(result)
    provenance = {
        "build": "12.1.0.69933",
        "build_key": "dcfc90fffd79ba00406ae46f5f657592",
        "file_data_id": 801681,
        "layout": "22331DAB",
        "db2_sha256": hashlib.sha256(data).hexdigest(),
        "csv_sha256": hashlib.sha256(output.read_bytes()).hexdigest(),
        "tdb_mount_offers_sha256": hashlib.sha256(json.dumps(offers).encode()).hexdigest(),
        "selected_offers": len(offers),
        "selected_costs": len(ids),
        "unavailable_currency_metadata": sorted(unavailable),
        "unavailable_item_metadata": sorted(unavailable_items),
        "currency_counts": "Raw DB2 precision; CurrencyTypes.Flags_0 & 0x8 divides by 100 exactly.",
    }
    output.with_suffix(".provenance.json").write_text(json.dumps(provenance, indent=2) + "\n")
    print(json.dumps(provenance, indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--world", type=Path, required=True)
    parser.add_argument("--db2", type=Path, required=True)
    parser.add_argument("--reader", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    export(args.world, args.db2, args.reader, args.output)


if __name__ == "__main__":
    main()
