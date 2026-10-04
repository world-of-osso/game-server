#!/usr/bin/env python3
"""Extract a Retail DB2 from the local CASC install and decode it to a wago-style CSV.

Used by `db2.py extract` for tables wago.tools must not be used for. The DB2 is read
by FileDataID with game-engine's `casc-local` (local CASC storage, never the CDN) and
decoded as WDC5. Column names and INTEGER/REAL types come from the table's DDL in
db2_schema.sql, positionally; the expected layout hash pins that order to the build
(WoWDBDefs `LAYOUT` line for the table).

Handles the subset the local tables need: unencrypted or already decrypted sections, inline
IDs or a non-inline ID list (prepended as the first column), a relationship map (its
foreign id appended as the last column), copy tables, and field
compression none/bitpacked/common/pallet/pallet array/signed bitpacked. A pallet-array
field expands to its array-count columns. String fields (TEXT columns) resolve
through global record offsets and the virtual concatenation of all section string
blocks. Zero-filled encrypted sections are reported and skipped; explicit selected
IDs must all resolve from readable records. Declared inline uint32 arrays are
supported; sparse (offset-map) layouts fail.
Tables with localized strings are read from their enUS copy (casc_locale.py).
"""

import os
import struct
import subprocess
import tempfile
from pathlib import Path

CASC_LOCAL = Path(
    os.environ.get(
        "CASC_LOCAL",
        "/syncthing/Sync/Projects/world-of-osso/asset-resolver/target/debug/casc-local",
    )
)
# casc-local resolves the listfile and TACT keys under <cwd>/data.
CASC_CWD = Path(
    os.environ.get("CASC_CWD", "/syncthing/Sync/Projects/world-of-osso/game-engine")
)

COMPRESSION_NONE, COMPRESSION_BITPACKED, COMPRESSION_COMMON, COMPRESSION_PALLET = (
    0,
    1,
    2,
    3,
)
COMPRESSION_PALLET_ARRAY, COMPRESSION_SIGNED = 4, 5


def extract_fdid(fdid, out_dir):
    """Path of FileDataID `fdid` extracted into out_dir by casc-local."""
    result = subprocess.run(
        [str(CASC_LOCAL), str(fdid), "-o", str(out_dir)],
        cwd=CASC_CWD,
        capture_output=True,
        text=True,
        check=False,
    )
    path = Path(out_dir) / f"{fdid}.db2"
    if result.returncode != 0 or not path.is_file():
        raise SystemExit(
            f"casc-local {fdid} failed ({result.returncode}): {result.stderr.strip()}"
        )
    return path


def read_header(data):
    if data[:4] != b"WDC5":
        raise ValueError(f"not WDC5: {data[:4]!r}")
    (
        record_count,
        field_count,
        record_size,
        _string_size,
        _table_hash,
        layout_hash,
        _min_id,
        _max_id,
        _locale,
    ) = struct.unpack_from("<9I", data, 136)
    flags, id_index = struct.unpack_from("<HH", data, 172)
    (
        _total_fields,
        _bitpacked_offset,
        _lookup_columns,
        info_size,
        common_size,
        pallet_size,
        section_count,
    ) = struct.unpack_from("<7I", data, 176)
    return {
        "record_count": record_count,
        "field_count": field_count,
        "record_size": record_size,
        "layout_hash": layout_hash,
        "flags": flags,
        "id_index": id_index,
        "info_size": info_size,
        "common_size": common_size,
        "pallet_size": pallet_size,
        "section_count": section_count,
    }


def decode(data, text_columns=frozenset(), *, record_ids=None, inline_arrays=None):
    """(header, {id: [raw uint32 per column]}) for every record, copies included.

    With a non-inline ID list the ID is prepended as column 0; a pallet-array field
    contributes its array-count values. Columns in `text_columns` hold the string
    their field's offset points at: the offset counts from the field's own byte in
    the record (fields before them must not be arrays)."""
    # Selected-row imports must resolve every requested ID from readable records.
    # Retain master decoding of decrypted sections and warnings for zero-filled ones.
    inline_arrays = inline_arrays or {}
    header = read_header(data)
    if header["flags"] & 1:
        raise ValueError("sparse (offset map) DB2 layout is not supported")
    offset = 204
    sections = []
    for _ in range(header["section_count"]):
        (
            key,
            file_offset,
            count,
            strings,
            _end,
            id_list,
            relations,
            _offset_map,
            copies,
        ) = struct.unpack_from("<Q8I", data, offset)
        sections.append((key, file_offset, count, strings, id_list, copies, relations))
        offset += 40
    offset += header["field_count"] * 4
    infos = [
        struct.unpack_from("<HHIIIII", data, offset + 24 * i)
        for i in range(header["info_size"] // 24)
    ]
    offset += header["info_size"]
    pallets, commons = {}, {}
    cursor = offset
    for index, (_, _, extra, compression, *_rest) in enumerate(infos):
        if compression in (COMPRESSION_PALLET, COMPRESSION_PALLET_ARRAY):
            pallets[index] = data[cursor : cursor + extra]
            cursor += extra
        elif compression not in (
            COMPRESSION_NONE,
            COMPRESSION_BITPACKED,
            COMPRESSION_COMMON,
            COMPRESSION_SIGNED,
        ):
            raise ValueError(
                f"field {index}: compression {compression} is not supported"
            )
    cursor = offset + header["pallet_size"]
    for index, (_, _, extra, compression, *_rest) in enumerate(infos):
        if compression == COMPRESSION_COMMON:
            commons[index] = dict(
                struct.iter_unpack("<II", data[cursor : cursor + extra])
            )
            cursor += extra
    size = header["record_size"]
    rows = {}
    # WDC5 string offsets refer to global record/string blocks, not physical
    # section offsets (DBCD.IO/Readers/WDC5Reader.cs).
    string_blocks = []
    previous_strings = 0
    for key, file_offset, count, strings, *_ in sections:
        start = file_offset + count * size
        string_blocks.append((previous_strings, data[start : start + strings]))
        previous_strings += strings
    previous_records = 0
    for key, file_offset, count, strings, id_list, copies, relations in sections:
        record_base = previous_records
        previous_records += count
        if key and not any(data[file_offset : file_offset + count * size]):
            import warnings

            warnings.warn(
                f"Skipping zero-filled DB2 section key {key:016X}, {count} records",
                stacklevel=2,
            )
            continue
        ids_at = file_offset + count * size + strings
        ids = struct.unpack_from(f"<{id_list // 4}I", data, ids_at) if id_list else None
        text_fields = [column - (1 if ids else 0) for column in text_columns]
        # Section order: records, strings, ID list, copy table, relationship map
        # (entry count, min id, max id, then (foreign id, record index) pairs).
        relation_at = ids_at + id_list + 8 * copies
        related = {}
        if relations:
            entries = struct.unpack_from("<I", data, relation_at)[0]
            related = {
                index: foreign
                for foreign, index in struct.iter_unpack(
                    "<II", data[relation_at + 12 : relation_at + 12 + 8 * entries]
                )
            }
        for record in range(count):
            bits = int.from_bytes(
                data[file_offset + record * size : file_offset + (record + 1) * size],
                "little",
            )
            fields = [
                field_value(index, info, bits, pallets)
                for index, info in enumerate(infos)
            ]
            record_id = ids[record] if ids else fields[header["id_index"]]
            for index in text_fields:
                if fields[index] == 0:
                    fields[index] = ""
                    continue
                string_offset = (
                    (record_base + record - header["record_count"]) * size
                    + infos[index][0] // 8
                    + fields[index]
                )
                fields[index] = read_global_string(string_blocks, string_offset)
            for index, count in inline_arrays.items():
                bit_offset, bit_size, _, compression, *_ = infos[index]
                if compression != COMPRESSION_NONE or bit_size != count * 32:
                    raise ValueError(
                        f"field {index}: expected inline array of {count} uint32 values"
                    )
                fields[index] = tuple(
                    (bits >> (bit_offset + slot * 32)) & 0xFFFFFFFF
                    for slot in range(count)
                )
            for index, info in enumerate(infos):
                if info[3] == COMPRESSION_COMMON:
                    fields[index] = commons[index].get(record_id, info[4])
            values = [record_id] if ids else []
            for value in fields:
                values.extend(value if isinstance(value, tuple) else (value,))
            if relations:
                values.append(related.get(record, 0))
            rows[record_id] = values
        id_column = 0 if ids else header["id_index"]
        copy_at = ids_at + id_list
        for new_id, source_id in struct.iter_unpack(
            "<II", data[copy_at : copy_at + 8 * copies]
        ):
            rows[new_id] = [
                new_id if i == id_column else v for i, v in enumerate(rows[source_id])
            ]
    if record_ids is not None:
        missing = set(record_ids) - rows.keys()
        if missing:
            raise ValueError(f"missing readable DB2 rows: {sorted(missing)}")
        rows = {record_id: rows[record_id] for record_id in sorted(record_ids)}
    return header, rows


def read_global_string(blocks, offset):
    for base, block in blocks:
        at = offset - base
        if 0 <= at < len(block):
            return block[at : block.index(b"\0", at)].decode("utf-8")
    raise ValueError(f"DB2 string offset {offset} is not in a readable string block")


def field_value(index, info, bits, pallets):
    bit_offset, bit_size, _, compression, _, value2, _ = info
    raw = (bits >> bit_offset) & ((1 << bit_size) - 1) if bit_size else 0
    if compression == COMPRESSION_PALLET:
        return struct.unpack_from("<I", pallets[index], raw * 4)[0]
    if compression == COMPRESSION_PALLET_ARRAY:
        count = info[6]
        return struct.unpack_from(f"<{count}I", pallets[index], raw * 4 * count)
    if compression == COMPRESSION_SIGNED and raw & (1 << (value2 - 1)):
        return raw - (1 << value2)
    return raw


def typed(raw, sql_type):
    if sql_type == "TEXT":
        return raw
    if sql_type == "REAL":
        # The exact float32 value (a 7-digit rendering loses up to 3 significant digits).
        return struct.unpack("<f", struct.pack("<I", raw & 0xFFFFFFFF))[0]
    return struct.unpack("<i", struct.pack("<I", raw & 0xFFFFFFFF))[0]


def extract_rows(fdid, layout_hash, columns, localized=False):
    """(column names, rows) of FileDataID `fdid`; columns = [(name, 'INTEGER'|'REAL'|'TEXT')] from the
    DDL. A localized table is read from its enUS copy with its TEXT columns' strings."""
    if localized:
        import casc_locale

        data = casc_locale.read_enus(fdid)
    else:
        with tempfile.TemporaryDirectory() as out_dir:
            data = extract_fdid(fdid, out_dir).read_bytes()
    text_columns = frozenset(
        i for i, (_, sql_type) in enumerate(columns) if sql_type == "TEXT"
    )
    header, rows = decode(data, text_columns if localized else frozenset())
    if header["layout_hash"] != layout_hash:
        raise SystemExit(
            f"FDID {fdid}: layout {header['layout_hash']:08X}, expected {layout_hash:08X}"
        )
    widths = {len(values) for values in rows.values()}
    if widths != {len(columns)}:
        raise SystemExit(
            f"FDID {fdid}: decoded {sorted(widths)} columns, schema has {len(columns)}"
        )
    if not localized and any(sql_type == "TEXT" for _, sql_type in columns):
        raise SystemExit(
            f"FDID {fdid}: string columns are read from localized tables only"
        )
    return [name for name, _ in columns], [
        [typed(raw, sql_type) for raw, (_, sql_type) in zip(rows[record_id], columns)]
        for record_id in sorted(rows)
    ]
