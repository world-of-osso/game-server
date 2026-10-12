#!/usr/bin/env python3
"""Add source SummonProperties flags to an existing DB, preserving every identity.

Local CASC69933 FDID1345276/layoutA4CA5ECF; TC a352b1fa DB2LoadInfo
columns ID/Control/Faction/Title/Slot/Flags1/Flags2. Three encrypted records
are omitted by db2_casc, never synthesized. Run canonical imports only as lead.
"""
import argparse
from pathlib import Path
import sqlite3

import db2_casc

LAYOUT = 0xA4CA5ECF
READABLE_RECORDS = 3610
UINT32_MASK = 0xFFFFFFFF


def signed32(value):
    return value - 0x100000000 if value >= 0x80000000 else value


def decode_source_records(payload):
    header = db2_casc.read_header(payload)
    if header['layout_hash'] != LAYOUT or header['field_count'] != 5:
        raise ValueError('SummonProperties layout must be A4CA5ECF/5 fields')
    _, records = db2_casc.decode(payload)
    if len(records) != READABLE_RECORDS:
        raise ValueError('Pinned69933 import requires all3610 readable SummonProperties records')
    if any(len(row) != 7 or row[0] != record_id for record_id, row in records.items()):
        raise ValueError('SummonProperties identity/expanded width differs')
    return {record_id: (record_id, *(signed32(x) for x in row[1:5]),
                        row[5] & UINT32_MASK, row[6] & UINT32_MASK)
            for record_id, row in records.items()}


def read_existing_source_ids(conn, source):
    rows = conn.execute('SELECT ID,Control,Faction,Title,Slot FROM summon_properties').fetchall()
    for row in rows:
        record = source.get(row[0])
        if record is None:
            raise ValueError(f'SummonProperties{row[0]} has no readable source record')
        # Legacy rows store some signed DB2 enums as unsigned32 (e.g.3001's
        # Title=-1). Compare the same source bits without rewriting that identity.
        identity = (row[0], *(signed32(x & UINT32_MASK) for x in row[1:]))
        if identity != record[:5]:
            raise ValueError(f'SummonProperties{row[0]} metadata differs from pinned source')
    return [row[0] for row in rows]


def add_flag_columns(conn):
    columns = {row[1]: row[2] for row in conn.execute('PRAGMA table_info(summon_properties)')}
    for name in ['Flags_0', 'Flags_1']:
        if name in columns:
            if columns[name].upper() != 'INTEGER':
                raise ValueError(f'SummonProperties {name} must be INTEGER')
        else:
            conn.execute(f'ALTER TABLE summon_properties ADD COLUMN {name} INTEGER')


def import_flags(conn, payload):
    source = decode_source_records(payload)
    ids = read_existing_source_ids(conn, source)
    conn.execute('SAVEPOINT summon_flag_import')
    try:
        add_flag_columns(conn)
        conn.executemany('UPDATE summon_properties SET Flags_0=?,Flags_1=? WHERE ID=?',
                         ((source[i][5], source[i][6], i) for i in ids))
    except Exception:
        conn.execute('ROLLBACK TO summon_flag_import')
        conn.execute('RELEASE summon_flag_import')
        raise
    conn.execute('RELEASE summon_flag_import')
    return len(ids)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--db2', type=Path, required=True)
    parser.add_argument('--db', type=Path, required=True)
    args = parser.parse_args()
    if not args.db.is_file():
        parser.error(f'database not found: {args.db}')
    with sqlite3.connect(args.db) as conn:
        count = import_flags(conn, args.db2.read_bytes())
    print(f'Imported sourced flags for {count} existing SummonProperties rows')


if __name__ == '__main__':
    main()
