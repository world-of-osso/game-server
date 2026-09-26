#!/usr/bin/env python3
"""Extract a Retail DB2 from the local CASC install and decode it to a wago-style CSV.

Used by `db2.py extract` for tables wago.tools must not be used for. The DB2 is read
by FileDataID with game-engine's `casc-local` (local CASC storage, never the CDN) and
decoded as WDC5. Column names and INTEGER/REAL types come from the table's DDL in
db2_schema.sql, positionally; the expected layout hash pins that order to the build
(WoWDBDefs `LAYOUT` line for the table).

Handles the subset the local tables need: one or more unencrypted sections, inline
IDs or an ID list, copy tables, and field compression none/bitpacked/common/pallet/
signed bitpacked. String fields, pallet arrays and sparse (offset-map) layouts fail.
"""

import os
import struct
import subprocess
import tempfile
from pathlib import Path

CASC_LOCAL = Path(os.environ.get("CASC_LOCAL", "/syncthing/Sync/Projects/world-of-osso/asset-resolver/target/debug/casc-local"))
# casc-local resolves the listfile and TACT keys under <cwd>/data.
CASC_CWD = Path(os.environ.get("CASC_CWD", "/syncthing/Sync/Projects/world-of-osso/game-engine"))

COMPRESSION_NONE, COMPRESSION_BITPACKED, COMPRESSION_COMMON, COMPRESSION_PALLET = 0, 1, 2, 3
COMPRESSION_SIGNED = 5


def extract_fdid(fdid, out_dir):
    """Path of FileDataID `fdid` extracted into out_dir by casc-local."""
    result = subprocess.run([str(CASC_LOCAL), str(fdid), "-o", str(out_dir)], cwd=CASC_CWD,
                            capture_output=True, text=True, check=False)
    path = Path(out_dir) / f"{fdid}.db2"
    if result.returncode != 0 or not path.is_file():
        raise SystemExit(f"casc-local {fdid} failed ({result.returncode}): {result.stderr.strip()}")
    return path


def read_header(data):
    if data[:4] != b"WDC5":
        raise ValueError(f"not WDC5: {data[:4]!r}")
    (record_count, field_count, record_size, _string_size, _table_hash, layout_hash, _min_id, _max_id,
     _locale) = struct.unpack_from("<9I", data, 136)
    flags, id_index = struct.unpack_from("<HH", data, 172)
    (_total_fields, _bitpacked_offset, _lookup_columns, info_size, common_size, pallet_size,
     section_count) = struct.unpack_from("<7I", data, 176)
    return dict(record_count=record_count, field_count=field_count, record_size=record_size,
                layout_hash=layout_hash, flags=flags, id_index=id_index, info_size=info_size,
                common_size=common_size, pallet_size=pallet_size, section_count=section_count)


def decode(data):
    """{id: [raw uint32 per field]} for every record, copies included."""
    header = read_header(data)
    if header["flags"] & 1:
        raise ValueError("sparse (offset map) DB2 layout is not supported")
    offset = 204
    sections = []
    for _ in range(header["section_count"]):
        key, file_offset, count, strings, _end, id_list, _rel, _offset_map, copies = struct.unpack_from("<Q8I", data, offset)
        if key:
            raise ValueError("encrypted DB2 section is not supported")
        sections.append((file_offset, count, strings, id_list, copies))
        offset += 40
    offset += header["field_count"] * 4
    infos = [struct.unpack_from("<HHIIIII", data, offset + 24 * i) for i in range(header["info_size"] // 24)]
    offset += header["info_size"]
    pallets, commons = {}, {}
    cursor = offset
    for index, (_, _, extra, compression, *_rest) in enumerate(infos):
        if compression == COMPRESSION_PALLET:
            pallets[index] = data[cursor:cursor + extra]
            cursor += extra
        elif compression not in (COMPRESSION_NONE, COMPRESSION_BITPACKED, COMPRESSION_COMMON, COMPRESSION_SIGNED):
            raise ValueError(f"field {index}: compression {compression} is not supported")
    cursor = offset + header["pallet_size"]
    for index, (_, _, extra, compression, *_rest) in enumerate(infos):
        if compression == COMPRESSION_COMMON:
            commons[index] = dict(struct.iter_unpack("<II", data[cursor:cursor + extra]))
            cursor += extra
    size = header["record_size"]
    rows = {}
    for file_offset, count, strings, id_list, copies in sections:
        ids_at = file_offset + count * size + strings
        ids = struct.unpack_from(f"<{id_list // 4}I", data, ids_at) if id_list else None
        for record in range(count):
            bits = int.from_bytes(data[file_offset + record * size:file_offset + (record + 1) * size], "little")
            values = [field_value(index, info, bits, pallets) for index, info in enumerate(infos)]
            record_id = ids[record] if ids else values[header["id_index"]]
            for index, info in enumerate(infos):
                if info[3] == COMPRESSION_COMMON:
                    values[index] = commons[index].get(record_id, info[4])
            if ids:
                values[header["id_index"]] = record_id
            rows[record_id] = values
        copy_at = ids_at + id_list
        for new_id, source_id in struct.iter_unpack("<II", data[copy_at:copy_at + 8 * copies]):
            rows[new_id] = [new_id if i == header["id_index"] else v for i, v in enumerate(rows[source_id])]
    return header, rows


def field_value(index, info, bits, pallets):
    bit_offset, bit_size, _, compression, value1, value2, _ = info
    raw = (bits >> bit_offset) & ((1 << bit_size) - 1) if bit_size else 0
    if compression == COMPRESSION_PALLET:
        return struct.unpack_from("<I", pallets[index], raw * 4)[0]
    if compression == COMPRESSION_SIGNED and raw & (1 << (value2 - 1)):
        return raw - (1 << value2)
    return raw


def typed(raw, sql_type):
    if sql_type == "REAL":
        return float(f"{struct.unpack('<f', struct.pack('<I', raw & 0xFFFFFFFF))[0]:.7g}")
    return struct.unpack("<i", struct.pack("<I", raw & 0xFFFFFFFF))[0]


def extract_rows(fdid, layout_hash, columns):
    """(column names, rows) of FileDataID `fdid`; columns = [(name, 'INTEGER'|'REAL')] from the DDL."""
    with tempfile.TemporaryDirectory() as out_dir:
        data = extract_fdid(fdid, out_dir).read_bytes()
    header, rows = decode(data)
    if header["layout_hash"] != layout_hash:
        raise SystemExit(f"FDID {fdid}: layout {header['layout_hash']:08X}, expected {layout_hash:08X}")
    if header["field_count"] != len(columns):
        raise SystemExit(f"FDID {fdid}: {header['field_count']} fields, schema has {len(columns)} columns")
    if any(sql_type == "TEXT" for _, sql_type in columns):
        raise SystemExit(f"FDID {fdid}: string columns are not supported")
    return [name for name, _ in columns], [
        [typed(raw, sql_type) for raw, (_, sql_type) in zip(rows[record_id], columns)] for record_id in sorted(rows)
    ]
