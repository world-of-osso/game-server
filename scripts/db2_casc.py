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
supported. Sparse + Index (flags 0x5) supports inline strings and schema-declared
numeric arrays/types; other sparse variants fail explicitly.
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


def decode(
    data,
    text_columns=frozenset(),
    *,
    record_ids=None,
    inline_arrays=None,
    field_types=None,
):
    """(header, {id: [raw uint32 per column]}) for every record, copies included.

    With a non-inline ID list the ID is prepended as column 0; a pallet-array field
    contributes its array-count values. Columns in `text_columns` hold the string
    their field's offset points at: the offset counts from the field's own byte in
    the record (fields before them must not be arrays).

    Sparse 0x5 reads strings inline; text_columns indexes stored fields plus the
    non-inline ID, before array flattening. inline_arrays indexes stored fields
    and declares element counts. Sparse field_types uses those same field indexes:
    INTEGER sign-extends the metadata width, REAL reinterprets float32, and omitted
    fields stay UNSIGNED. Types/arrays must come from the matching DBD schema."""
    # Selected-row imports must resolve every requested ID from readable records.
    # Retain master decoding of decrypted sections and warnings for zero-filled ones.
    inline_arrays = inline_arrays or {}
    header = read_header(data)
    if header["flags"] & 1:
        return header, decode_sparse(
            data, header, text_columns, inline_arrays, field_types or {}, record_ids
        )
    if field_types:
        raise ValueError("field_types is supported only for sparse DB2 layouts")
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
    return header, select_rows(rows, record_ids)


def select_rows(rows, record_ids):
    if record_ids is None:
        return rows
    missing = set(record_ids) - rows.keys()
    if missing:
        raise ValueError(f"missing readable DB2 rows: {sorted(missing)}")
    return {record_id: rows[record_id] for record_id in sorted(record_ids)}


def decode_sparse(data, header, text_columns, arrays, field_types, record_ids):
    """Observed WDC5 0x5: record bytes, IDs, copies, offsets, relations, sparse IDs."""
    if header["flags"] != 5:
        raise ValueError(f"sparse DB2 flags {header['flags']:#x} are not supported")
    sections = [
        struct.unpack_from("<Q8I", data, 204 + 40 * i)
        for i in range(header["section_count"])
    ]
    if record_ids is None and any(section[0] for section in sections):
        raise ValueError("encrypted DB2 section is not supported")
    offset = 204 + 40 * header["section_count"]
    metadata = [
        struct.unpack_from("<hH", data, offset + 4 * i)
        for i in range(header["field_count"])
    ]
    offset += 4 * header["field_count"]
    if header["info_size"] != 24 * len(metadata):
        raise ValueError("sparse DB2 field metadata count mismatch")
    infos = [
        struct.unpack_from("<HHIIIII", data, offset + 24 * i)
        for i in range(len(metadata))
    ]
    for index, info in enumerate(infos):
        if info[3] != COMPRESSION_NONE:
            raise ValueError(
                f"sparse field {index}: compression {info[3]} is not supported"
            )
    rows, copy_pairs = {}, []
    for (
        key,
        start,
        count,
        strings,
        end,
        id_size,
        relations,
        map_count,
        copies,
    ) in sections:
        if key:
            continue
        if strings or map_count != count or id_size not in (0, 4 * count):
            raise ValueError("unsupported sparse DB2 section shape")
        tail_end = (
            end + id_size + 8 * copies + 6 * map_count + relations + 4 * map_count
        )
        if not (0 <= start <= end <= tail_end <= len(data)):
            raise ValueError("truncated sparse DB2 section")
        copy_at = end + id_size
        copy_pairs.extend(
            struct.iter_unpack("<II", data[copy_at : copy_at + 8 * copies])
        )
        map_at = copy_at + 8 * copies
        entries = struct.iter_unpack("<IH", data[map_at : map_at + 6 * map_count])
        relation_at = map_at + 6 * map_count
        related = read_sparse_relations(data, relation_at, relations, count)
        ids_at = relation_at + relations
        ids = struct.unpack_from(f"<{map_count}I", data, ids_at)
        for index, (record_id, (at, size)) in enumerate(zip(ids, entries)):
            if not size or not start <= at < at + size <= end:
                raise ValueError(f"sparse DB2 row {record_id}: invalid offset/size")
            values = read_sparse_row(
                data[at : at + size], metadata, infos, text_columns, arrays, field_types
            )
            rows[record_id] = [record_id, *values]
            if relations:
                rows[record_id].append(related.get(index, 0))
    for new_id, source_id in copy_pairs:
        if source_id not in rows:
            raise ValueError(
                f"sparse DB2 copy {new_id}: missing readable source {source_id}"
            )
        rows[new_id] = [new_id, *rows[source_id][1:]]
    return select_rows(rows, record_ids)


def read_sparse_relations(data, at, size, count):
    if not size:
        return {}
    if size < 12:
        raise ValueError("truncated sparse DB2 relationship map")
    entries = struct.unpack_from("<I", data, at)[0]
    if size != 12 + 8 * entries:
        raise ValueError("invalid sparse DB2 relationship map size")
    related = {
        index: foreign
        for foreign, index in struct.iter_unpack("<II", data[at + 12 : at + size])
    }
    if any(index >= count for index in related):
        raise ValueError("invalid sparse DB2 relationship record index")
    return related


def read_sparse_row(data, metadata, infos, text_columns, arrays, field_types):
    values, cursor = [], 0
    for index, ((bits, _), info) in enumerate(zip(metadata, infos)):
        if index + 1 in text_columns:
            end = data.find(b"\0", cursor)
            if end < 0:
                raise ValueError(f"sparse field {index}: unterminated inline string")
            values.append(data[cursor:end].decode("utf-8"))
            cursor = end + 1
            continue
        width = 32 - bits
        if width <= 0:
            width = info[5]
        count = arrays.get(index, 1)
        if width not in (8, 16, 32) or count <= 0 or info[1] != width * count:
            raise ValueError(
                f"sparse field {index}: unsupported width/array declaration"
            )
        for _ in range(count):
            end = cursor + width // 8
            if end > len(data):
                raise ValueError(f"sparse field {index}: truncated sparse row")
            raw = int.from_bytes(data[cursor:end], "little")
            values.append(sparse_typed(raw, width, field_types.get(index, "UNSIGNED")))
            cursor = end
    # Observed ItemSparse rows have up to three zero bytes of 4-byte alignment.
    if cursor != len(data) and (
        len(data) != (cursor + 3) // 4 * 4 or any(data[cursor:])
    ):
        raise ValueError(
            f"sparse row has {len(data) - cursor} undeclared trailing bytes"
        )
    return values


def sparse_typed(raw, width, field_type):
    if field_type == "UNSIGNED":
        return raw
    if field_type == "INTEGER":
        return raw - (1 << width) if raw & (1 << (width - 1)) else raw
    if field_type == "REAL" and width == 32:
        return typed(raw, "REAL")
    raise ValueError(f"unsupported sparse field type {field_type!r} at width {width}")


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
