#!/usr/bin/env python3
"""Read the enUS copy of a locale-split file from the local CASC install.

DB2s with localized strings (Difficulty.db2) have one root entry per locale;
casc-local picks the last one, which the local install does not hold. This reads
the local build's root.bin (the asset-resolver cache for the active `.build.info`
build key), keeps the entry whose locale mask has enUS, maps its content key to an
encoding key through encoding.bin and reads that key's BLTE from the install's
`Data/data` archives (newest index per bucket). Only plain (N) and zlib (Z) BLTE
chunks are handled; encrypted chunks fail. Local storage only, never the CDN.
"""

import hashlib
import struct
import zlib
from pathlib import Path

INSTALL = Path("/syncthing/World of Warcraft")
CACHE = Path.home() / ".cache/asset-resolver/casc/wow"
LOCALE_ENUS = 0x2


def active_build_key():
    lines = (INSTALL / ".build.info").read_text().splitlines()
    header = [column.split("!")[0] for column in lines[0].split("|")]
    for line in lines[1:]:
        row = dict(zip(header, line.split("|")))
        if row["Product"] == "wow" and row["Active"] == "1":
            return row["Build Key"]
    raise SystemExit("no active wow build in .build.info")


def root_content_key(root, fdid, locale):
    if root[:4] != b"TSFM":
        raise SystemExit(f"root.bin: not TSFM ({root[:4]!r})")
    header_size, version, _total, named = struct.unpack_from("<4I", root, 4)
    pos = header_size
    while pos < len(root):
        count = struct.unpack_from("<I", root, pos)[0]
        block_locale, flags1, flags2 = struct.unpack_from("<3I", root, pos + 4)
        content_flags = flags1 | flags2 | (root[pos + 16] << 17)
        pos += 17 if version < 4 else 18
        deltas = struct.unpack_from(f"<{count}i", root, pos)
        pos += 4 * count
        keys_at = pos
        pos += 16 * count
        if not content_flags & 0x10000000 and named:
            pos += 8 * count
        if not block_locale & locale:
            continue
        file_id = -1
        for index, delta in enumerate(deltas):
            file_id += 1 + delta
            if file_id == fdid:
                return root[keys_at + 16 * index:keys_at + 16 * index + 16]
    raise SystemExit(f"FDID {fdid} has no root entry for locale {locale:#x}")


def encoding_key(encoding, content_key):
    if encoding[:2] != b"EN":
        raise SystemExit("encoding.bin: not EN")
    content_page_kb = struct.unpack_from(">H", encoding, 5)[0]
    content_pages = struct.unpack_from(">I", encoding, 9)[0]
    spec_size = struct.unpack_from(">I", encoding, 18)[0]
    pos = 22 + spec_size
    first_keys = [encoding[pos + 32 * i:pos + 32 * i + 16] for i in range(content_pages)]
    pos += 32 * content_pages
    page = max(i for i, key in enumerate(first_keys) if key <= content_key)
    cursor = pos + page * content_page_kb * 1024
    end = cursor + content_page_kb * 1024
    while cursor < end and encoding[cursor]:
        key_count = encoding[cursor]
        if encoding[cursor + 6:cursor + 22] == content_key:
            return encoding[cursor + 22:cursor + 38]
        cursor += 22 + 16 * key_count
    raise SystemExit(f"content key {content_key.hex()} not in encoding.bin")


def archive_location(ekey):
    newest = {}
    for index in (INSTALL / "Data/data").glob("*.idx"):
        bucket, generation = index.stem[:2], int(index.stem[2:], 16)
        if bucket not in newest or generation > newest[bucket][0]:
            newest[bucket] = (generation, index)
    for _, index in newest.values():
        data = index.read_bytes()
        pos = (8 + struct.unpack_from("<I", data, 0)[0] + 15) & ~15
        size = struct.unpack_from("<I", data, pos)[0]
        pos += 8
        for entry in range(pos, pos + size, 18):
            if data[entry:entry + 9] == ekey[:9]:
                offset = int.from_bytes(data[entry + 9:entry + 14], "big")
                return offset >> 30, offset & ((1 << 30) - 1), struct.unpack_from("<I", data, entry + 14)[0]
    raise SystemExit(f"encoding key {ekey.hex()} not in the local archives")


def decode_blte(blte):
    if blte[:4] != b"BLTE":
        raise SystemExit("not BLTE")
    header_size = struct.unpack_from(">I", blte, 4)[0]
    count = int.from_bytes(blte[9:12], "big")
    out, pos = bytearray(), header_size
    for i in range(count):
        compressed, decompressed, md5 = struct.unpack_from(">II16s", blte, 12 + 24 * i)
        chunk = blte[pos:pos + compressed]
        pos += compressed
        if hashlib.md5(chunk).digest() != md5:
            raise SystemExit(f"BLTE chunk {i} checksum mismatch")
        mode = chunk[:1]
        if mode == b"N":
            data = chunk[1:]
        elif mode == b"Z":
            data = zlib.decompress(chunk[1:])
        else:
            raise SystemExit(f"BLTE chunk {i}: unsupported mode {mode!r}")
        if len(data) != decompressed:
            raise SystemExit(f"BLTE chunk {i}: {len(data)} bytes, expected {decompressed}")
        out += data
    return bytes(out)


def read_enus(fdid):
    """The bytes of FileDataID `fdid`'s enUS copy."""
    cache = CACHE / active_build_key()
    content_key = root_content_key((cache / "root.bin").read_bytes(), fdid, LOCALE_ENUS)
    ekey = encoding_key((cache / "encoding.bin").read_bytes(), content_key)
    archive, offset, size = archive_location(ekey)
    with open(INSTALL / f"Data/data/data.{archive:03d}", "rb") as handle:
        handle.seek(offset)
        raw = handle.read(size)
    # A 30-byte archive record header precedes the BLTE.
    return decode_blte(raw[30:])
