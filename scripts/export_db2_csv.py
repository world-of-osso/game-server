#!/usr/bin/env python3
"""Export CraftingOrder FDID4529122, Retail12.1.0.69933 layout05F8699D.

Field order is the exact local WoWDBDefs CraftingOrder.dbd layout. No unknown
field is interpreted as a fee, duration or eligibility condition.
Usage: scripts/export_db2_csv.py CraftingOrder input.db2 output.csv
"""
import csv
import sys
from pathlib import Path

import db2_casc

COLUMNS = ('ID', 'SkillLineAbilityID', 'CraftingOrderHouseID', 'Field_10_0_2_46479_002')


def crafting_order_rows(data):
    if len(data) < 204 or data[:4] != b'WDC5':
        raise ValueError('CraftingOrder requires a WDC5 payload')
    header = db2_casc.read_header(data)
    if header['layout_hash'] != 0x05F8699D:
        raise ValueError('CraftingOrder layout must be 05F8699D')
    _, decoded = db2_casc.decode(data)
    rows = []
    for record_id, values in sorted(decoded.items()):
        if len(values) != 4 or values[0] != record_id or record_id <= 0:
            raise ValueError(f'CraftingOrder {record_id}: invalid decoded width/identity')
        rows.append(values)
    if header['record_count'] != 2047 or len(rows) != 2047:
        raise ValueError('CraftingOrder requires all 2047 authored rows')
    return rows


def main():
    if len(sys.argv) != 4 or sys.argv[1] != 'CraftingOrder':
        raise SystemExit(__doc__)
    rows = crafting_order_rows(Path(sys.argv[2]).read_bytes())
    with Path(sys.argv[3]).open('x', newline='') as output:
        writer = csv.writer(output)
        writer.writerow(COLUMNS)
        writer.writerows(rows)
    print(f'Exported {len(rows)} CraftingOrder rows')


if __name__ == '__main__':
    main()
