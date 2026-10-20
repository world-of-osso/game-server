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
numeric arrays/types; other sparse variants fail explicitly. Non-sparse inline arrays
use their declared count and metadata element width (8, 16 or 32 bits).
Tables with localized strings are read from their enUS copy (casc_locale.py).
"""

import os
import struct
import subprocess
import tempfile
import warnings
from collections import namedtuple
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
    relationship_column=None,
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
    fields stay UNSIGNED. Types/arrays must come from the matching DBD schema.
    relationship_column declares an inline parent column in the expanded output;
    the relationship map verifies that column instead of appending a duplicate.
    A non-inline parent (no column declared) is appended as the last column."""
    # Selected-row imports must resolve every requested ID from readable records.
    # Retain master decoding of decrypted sections and warnings for zero-filled ones.
    inline_arrays = inline_arrays or {}
    header = read_header(data)
    if header["flags"] & 1:
        if relationship_column is not None:
            raise ValueError(
                "inline relationship validation is not supported for sparse DB2"
            )
        return header, decode_sparse(
            data, header, text_columns, inline_arrays, field_types or {}, record_ids
        )
    if field_types:
        raise ValueError("field_types is supported only for sparse DB2 layouts")
    header, infos, sections = read_fixed(data)
    blocks = string_blocks(data, header)
    rows = {}
    for section in sections:
        if section.records is None:
            warnings.warn(
                f"Skipping zero-filled DB2 section key {section.key:016X}, {section.count} records",
                stacklevel=2,
            )
            continue
        text_fields = [
            column - (1 if section.id_list else 0) for column in text_columns
        ]
        for record in section.records:
            fields = list(record.values)
            for index in text_fields:
                fields[index] = read_string_at(
                    data, header, blocks, record.index, infos[index], fields[index]
                )
            for index, count in inline_arrays.items():
                bit_offset, bit_size, _, compression, *_ = infos[index]
                width, remainder = divmod(bit_size, count)
                if (
                    compression != COMPRESSION_NONE
                    or remainder
                    or width not in (8, 16, 32)
                ):
                    raise ValueError(
                        f"field {index}: expected inline array of {count} 8/16/32-bit values"
                    )
                fields[index] = tuple(
                    (record.bits >> (bit_offset + slot * width)) & ((1 << width) - 1)
                    for slot in range(count)
                )
            values = [record.id] if section.id_list else []
            for value in fields:
                values.extend(value if isinstance(value, tuple) else (value,))
            if section.relations:
                values = join_relationship(
                    values, record.parent or 0, relationship_column
                )
            rows[record.id] = values
        id_column = 0 if section.id_list else header["id_index"]
        for new_id, source_id in section.copies:
            rows[new_id] = [
                new_id if i == id_column else v for i, v in enumerate(rows[source_id])
            ]
    return header, select_rows(rows, record_ids)


def join_relationship(values, foreign_id, inline_column):
    if inline_column is None:
        return [*values, foreign_id]
    if not 0 <= inline_column < len(values):
        raise ValueError(
            f"inline relationship column {inline_column} is outside the row"
        )
    if values[inline_column] != foreign_id:
        raise ValueError(
            f"inline relationship {values[inline_column]} differs from mapped parent {foreign_id}"
        )
    return values


def select_rows(rows, record_ids):
    if record_ids is None:
        return rows
    missing = set(record_ids) - rows.keys()
    if missing:
        raise ValueError(f"missing readable DB2 rows: {sorted(missing)}")
    return {record_id: rows[record_id] for record_id in sorted(record_ids)}


def decode_sparse(data, header, text_columns, arrays, field_types, record_ids):
    """Observed WDC5 0x5: record bytes, IDs, copies, offsets, relations, sparse IDs."""
    # Local CollectableSource*Sparse 0x7 puts offset-map IDs before relations.
    if header["flags"] not in (5, 7):
        raise ValueError(f"sparse DB2 flags {header['flags']:#x} are not supported")
    sections = [
        struct.unpack_from("<Q8I", data, 204 + 40 * i)
        for i in range(header["section_count"])
    ]
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
        if key and count and not any(data[start:end]):
            import warnings

            warnings.warn(
                f"Skipping zero-filled DB2 section key {key:016X}, {count} records",
                stacklevel=2,
            )
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
        map_ids_at = map_at + 6 * map_count
        if header["flags"] & 2:
            ids_at = map_ids_at
            relation_at = ids_at + 4 * map_count
        else:
            relation_at = map_ids_at
            ids_at = relation_at + relations
        ids = struct.unpack_from(f"<{map_count}I", data, ids_at)
        relation_ids = set(ids) if header["flags"] & 2 else None
        related = read_sparse_relations(
            data, relation_at, relations, count, relation_ids
        )
        for index, (record_id, (at, size)) in enumerate(zip(ids, entries)):
            if not size or not start <= at < at + size <= end:
                raise ValueError(f"sparse DB2 row {record_id}: invalid offset/size")
            try:
                values = read_sparse_row(
                    data[at : at + size],
                    metadata,
                    infos,
                    text_columns,
                    arrays,
                    field_types,
                )
            except UnicodeDecodeError as error:
                import warnings

                warnings.warn(
                    f"unreadable UTF-8 in DB2 row {record_id}: {error}", stacklevel=2
                )
                header.setdefault("unreadable_record_ids", []).append(record_id)
                continue
            except ValueError as error:
                raise ValueError(
                    f"sparse DB2 row {record_id} in section {key:016X}: {error}"
                ) from error
            rows[record_id] = [record_id, *values]
            if relations:
                relation_key = record_id if header["flags"] & 2 else index
                rows[record_id].append(related.get(relation_key, 0))
    for new_id, source_id in copy_pairs:
        if source_id not in rows:
            if source_id in header.get("unreadable_record_ids", []):
                import warnings

                warnings.warn(
                    f"DB2 copy {new_id} has unreadable source {source_id}", stacklevel=2
                )
                header["unreadable_record_ids"].append(new_id)
                continue
            raise ValueError(
                f"sparse DB2 copy {new_id}: missing readable source {source_id}"
            )
        rows[new_id] = [new_id, *rows[source_id][1:]]
    return select_rows(rows, record_ids)


def read_sparse_relations(data, at, size, count, record_ids=None):
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
    invalid = (
        any(index not in record_ids for index in related)
        if record_ids is not None
        else any(index >= count for index in related)
    )
    if invalid:
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


FixedRecord = namedtuple("FixedRecord", "id values bits parent offset index")
FixedSection = namedtuple("FixedSection", "key count id_list relations records copies")


def read_fixed(data, id_field=None):
    """(header, field infos, sections) of a fixed-record WDC5, decoded once for every reader.

    Record values apply pallet, pallet-array, signed and common storage; inline arrays
    stay packed in each record's `bits`. A zero-filled encrypted section has
    `records=None` and no copies. Without an ID list, `id_field` (default: the header's
    id_index) names the inline ID field. `parent` is None for records the relationship
    map omits; `index` counts records across all sections.
    """
    header = read_header(data)
    size = header["record_size"]
    if id_field is None:
        id_field = header["id_index"]
    section_headers = [
        struct.unpack_from("<Q8I", data, 204 + 40 * i)
        for i in range(header["section_count"])
    ]
    offset = 204 + 40 * header["section_count"] + header["field_count"] * 4
    infos = [
        struct.unpack_from("<HHIIIII", data, offset + 24 * i)
        for i in range(header["info_size"] // 24)
    ]
    pallets, commons = read_field_storage(
        data, header, infos, offset + header["info_size"]
    )
    sections, record_base = [], 0
    for (
        key,
        start,
        count,
        strings,
        _,
        id_size,
        relation_size,
        _,
        copies,
    ) in section_headers:
        payload = data[start : start + count * size]
        if key and count and not any(payload):
            sections.append(
                FixedSection(key, count, bool(id_size), bool(relation_size), None, ())
            )
            record_base += count
            continue
        ids_at = start + count * size + strings
        ids = struct.unpack_from(f"<{id_size // 4}I", data, ids_at) if id_size else None
        copy_at = ids_at + id_size
        related = read_relations(data, copy_at + 8 * copies, relation_size)
        records = []
        for record in range(count):
            bits = int.from_bytes(
                payload[record * size : (record + 1) * size], "little"
            )
            values = [
                field_value(index, info, bits, pallets)
                for index, info in enumerate(infos)
            ]
            record_id = ids[record] if ids else values[id_field]
            for index, common in commons.items():
                values[index] = common.get(record_id, infos[index][4])
            records.append(
                FixedRecord(
                    record_id,
                    values,
                    bits,
                    related.get(record),
                    start + record * size,
                    record_base + record,
                )
            )
        pairs = tuple(struct.iter_unpack("<II", data[copy_at : copy_at + 8 * copies]))
        sections.append(
            FixedSection(key, count, bool(id_size), bool(relation_size), records, pairs)
        )
        record_base += count
    return header, infos, sections


def read_field_storage(data, header, infos, start):
    """Per-field pallet bytes and common {record id: value} maps.

    Scalar pallets precede array pallets in the pallet block, each group in field
    order. PlayerCondition 1D743D99 declares an array pallet before scalar ones: read
    in plain field order its deduplicated ContentTuningID pallet repeats entries and
    every row reads WeaponSubclassMask 1048866 / ContentTuningID 958 instead of 0.
    """
    for index, (_, _, _, compression, *_rest) in enumerate(infos):
        if compression not in (
            COMPRESSION_NONE,
            COMPRESSION_BITPACKED,
            COMPRESSION_COMMON,
            COMPRESSION_PALLET,
            COMPRESSION_PALLET_ARRAY,
            COMPRESSION_SIGNED,
        ):
            raise ValueError(
                f"field {index}: compression {compression} is not supported"
            )
    pallets, cursor = {}, start
    for kind in (COMPRESSION_PALLET, COMPRESSION_PALLET_ARRAY):
        for index, info in enumerate(infos):
            if info[3] == kind:
                pallets[index] = data[cursor : cursor + info[2]]
                cursor += info[2]
    commons, cursor = {}, start + header["pallet_size"]
    for index, info in enumerate(infos):
        if info[3] == COMPRESSION_COMMON:
            commons[index] = dict(
                struct.iter_unpack("<II", data[cursor : cursor + info[2]])
            )
            cursor += info[2]
    return pallets, commons


def read_relations(data, at, size):
    """{record index in section: foreign id} from a section relationship map."""
    if not size:
        return {}
    entries = struct.unpack_from("<I", data, at)[0]
    return {
        index: foreign
        for foreign, index in struct.iter_unpack(
            "<II", data[at + 12 : at + 12 + 8 * entries]
        )
    }


def string_blocks(data, header):
    """(string block offset, size, unreadable key or 0) per section, in file order."""
    size = header["record_size"]
    blocks = []
    for i in range(header["section_count"]):
        key, start, count, strings, *_ = struct.unpack_from("<Q8I", data, 204 + 40 * i)
        block = start + count * size
        unreadable = key if key and not any(data[start : block + strings]) else 0
        blocks.append((block, strings, unreadable))
    return blocks


def read_string_at(data, header, blocks, record_index, field, value):
    """String `value` bytes past `field`'s byte in global record `record_index`.

    WDC5 offsets address the virtual concatenation of every section's string block,
    counted from the end of all records (DBCD.IO/Readers/WDC5Reader.cs). A string in a
    zero-filled encrypted section fails instead of reading as empty."""
    if value == 0:
        return ""
    logical = (
        (record_index - header["record_count"]) * header["record_size"]
        + field[0] // 8
        + value
    )
    for block, strings, unreadable in blocks:
        if 0 <= logical < strings:
            if unreadable:
                raise ValueError(
                    f"string offset lands in unreadable encrypted section {unreadable:016X}"
                )
            end = data.index(b"\0", block + logical, block + strings)
            return data[block + logical : end].decode("utf-8")
        logical -= strings
    raise ValueError("string offset outside concatenated section string blocks")


def read_string(data, record_offset, field, value):
    """String field `value` of the record stored at byte `record_offset`."""
    header = read_header(data)
    size = header["record_size"]
    preceding = 0
    for i in range(header["section_count"]):
        _, start, count, *_ = struct.unpack_from("<Q8I", data, 204 + 40 * i)
        if start <= record_offset < start + count * size:
            index = preceding + (record_offset - start) // size
            break
        preceding += count
    else:
        raise ValueError(f"string record offset {record_offset} outside all sections")
    return read_string_at(
        data, header, string_blocks(data, header), index, field, value
    )


def field_value(index, info, bits, pallets):
    bit_offset, bit_size, _, compression, *_ = info
    raw = (bits >> bit_offset) & ((1 << bit_size) - 1) if bit_size else 0
    if compression == COMPRESSION_PALLET:
        return struct.unpack_from("<I", pallets[index], raw * 4)[0]
    if compression == COMPRESSION_PALLET_ARRAY:
        count = info[6]
        return struct.unpack_from(f"<{count}I", pallets[index], raw * 4 * count)
    if compression == COMPRESSION_SIGNED and bit_size and raw & (1 << (bit_size - 1)):
        return raw - (1 << bit_size)
    return raw


def typed(raw, sql_type):
    if sql_type == "TEXT":
        return raw
    if sql_type == "REAL":
        # The exact float32 value (a 7-digit rendering loses up to 3 significant digits).
        return struct.unpack("<f", struct.pack("<I", raw & 0xFFFFFFFF))[0]
    return struct.unpack("<i", struct.pack("<I", raw & 0xFFFFFFFF))[0]


def extract_rows(fdid, layout_hash, columns, localized=False, inline_arrays=None):
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
    header, rows = decode(
        data, text_columns if localized else frozenset(), inline_arrays=inline_arrays
    )
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
