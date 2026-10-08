"""Decode pinned Forever Creature / CreatureXDisplayInfo readable sections.

WoWDBDefs hash-matched layouts and source hashes: scripts/reference/forever-1.60.1.70205/README.md.
Encrypted sections are reported, never presented as decoded or complete data.
"""

import hashlib
import struct
from pathlib import Path

import db2_casc

DEFAULT_SOURCE = Path(__file__).resolve().parent / "reference/forever-1.60.1.70205"
SOURCES = {
    841631: (
        0x6E14C900,
        "772706ff900ff31227a0b61f39ec5073f038d48514def9f4f2fad03ae1cc070a",
    ),
    1864302: (
        0x2EA19FCF,
        "9b803b84e2fe4291a8e49f8a372490c4f0e06bbb389ac926a2bb3228a3cf42bb",
    ),
}


def readable_ids(data, header):
    """Read non-inline ID lists and copy IDs; retain original section string offsets."""
    ids, encrypted = set(), 0
    for index in range(header["section_count"]):
        key, offset, count, strings, _, id_bytes, _, _, copies = struct.unpack_from(
            "<Q8I", data, 204 + 40 * index
        )
        if key:
            encrypted += 1
            continue
        at = offset + count * header["record_size"] + strings
        ids.update(struct.unpack_from(f"<{id_bytes // 4}I", data, at))
        copy_at = at + id_bytes
        ids.update(
            new
            for new, _ in struct.iter_unpack(
                "<II", data[copy_at : copy_at + 8 * copies]
            )
        )
    return ids, encrypted


def decode_file(source, fdid, text_columns=frozenset()):
    path = source / f"{fdid}.db2"
    data = path.read_bytes()
    layout, digest = SOURCES[fdid]
    if hashlib.sha256(data).hexdigest() != digest:
        raise ValueError(f"{path}: SHA-256 mismatch")
    header = db2_casc.read_header(data)
    if header["layout_hash"] != layout:
        raise ValueError(f"{path}: expected layout {layout:08X}")
    ids, encrypted = readable_ids(data, header)
    _, rows = db2_casc.decode(data, text_columns, record_ids=ids)
    return rows, encrypted


def creature_row(values):
    if len(values) != 20:
        raise ValueError(f"Creature {values[0]}: expected 20 expanded fields")
    models = [
        {
            "display_id": display,
            "probability": db2_casc.typed(probability, "REAL"),
            "scale": 1.0,
        }
        for display, probability in zip(values[9:13], values[13:17])
        if display
    ]
    return {
        "name": values[1],
        "title": values[3],
        "classification": values[5],
        "type": values[6],
        "family": values[7],
        "models": models,
    }


def load_catalog(source=DEFAULT_SOURCE):
    creatures, encrypted_creatures = decode_file(
        source, 841631, frozenset({1, 2, 3, 4})
    )
    displays, encrypted_displays = decode_file(source, 1864302)
    catalog = {entry: creature_row(values) for entry, values in creatures.items()}
    related = {}
    for values in displays.values():
        if len(values) != 7:
            raise ValueError(f"CreatureXDisplayInfo {values[0]}: expected 7 fields")
        related.setdefault(values[6], []).append(
            (
                values[4],
                {
                    "display_id": values[1],
                    "probability": db2_casc.typed(values[2], "REAL"),
                    "scale": db2_casc.typed(values[3], "REAL"),
                },
            )
        )
    for entry, models in related.items():
        record = catalog.setdefault(entry, {"models": []})
        record["models"] = [
            model for _, model in sorted(models, key=lambda pair: pair[0])
        ]
    return catalog, {
        "creature_rows": len(creatures),
        "xdisplay_rows": len(displays),
        "encrypted_sections": {
            "841631": encrypted_creatures,
            "1864302": encrypted_displays,
        },
    }
