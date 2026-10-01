#!/usr/bin/env python3
"""Sample root ADT terrain (not WMO floors or water) at Retail world X/Y.

Tile row/column are the two numbers in e.g. azeroth_40_15.adt, not MCNK
indices. Matches game-engine's adt_format/adt.rs and terrain_height_data.rs:
MCNK index_y advances world X rows; index_x advances world Y columns.
"""

import argparse
import math
import struct
from pathlib import Path

CHUNK_SIZE = 100.0 / 3.0
UNIT_SIZE = CHUNK_SIZE / 8.0
TILE_SIZE = CHUNK_SIZE * 16.0


def chunks(data):
    """Read reversed-tag ADT chunks, rejecting incomplete frames."""
    offset = 0
    while offset < len(data):
        if len(data) - offset < 8:
            raise ValueError(f"truncated chunk header at byte {offset}")
        tag = data[offset : offset + 4][::-1]
        size = struct.unpack_from("<I", data, offset + 4)[0]
        end = offset + 8 + size
        if end > len(data):
            raise ValueError(f"truncated {tag!r} payload at byte {offset}")
        yield tag, data[offset + 8 : end]
        offset = end


def interpolate_height(heights, row, col, fx, fz):
    """Engine's four-triangle fan, with barycentric weights."""
    tl = heights[row * 17 + col]
    tr = heights[row * 17 + col + 1]
    bl = heights[(row + 1) * 17 + col]
    br = heights[(row + 1) * 17 + col + 1]
    center = heights[row * 17 + 9 + col]
    dx, dz = fx - 0.5, fz - 0.5
    if abs(dz) >= abs(dx):
        if dz < 0:
            a, b = (0, 0, tl), (1, 0, tr)
        else:
            a, b = (1, 1, br), (0, 1, bl)
    elif dx > 0:
        a, b = (1, 0, tr), (1, 1, br)
    else:
        a, b = (0, 1, bl), (0, 0, tl)
    ax, az, ha = a
    bx, bz, hb = b
    det = (bz - 0.5) * (ax - 0.5) + (0.5 - bx) * (az - 0.5)
    wa = ((bz - 0.5) * (fx - 0.5) + (0.5 - bx) * (fz - 0.5)) / det
    wb = ((0.5 - az) * (fx - 0.5) + (ax - 0.5) * (fz - 0.5)) / det
    return wa * ha + wb * hb + (1 - wa - wb) * center


def sample_adt_height(data, world_x, world_y, tile_row, tile_col):
    """Sample one point; reject wrong tile/chunk origins and absent/bad heights."""
    if not math.isfinite(world_x) or not math.isfinite(world_y):
        raise ValueError("world coordinates must be finite")
    if not (0 <= tile_row < 64 and 0 <= tile_col < 64):
        raise ValueError("tile row/column must be in 0..63")
    tile_x = (32 - tile_col) * TILE_SIZE
    tile_y = (32 - tile_row) * TILE_SIZE
    local_x, local_y = tile_x - world_x, tile_y - world_y
    if not (0 <= local_x < TILE_SIZE and 0 <= local_y < TILE_SIZE):
        raise ValueError("world coordinates do not match requested tile")
    wanted_y = math.floor(local_x / CHUNK_SIZE)
    wanted_x = math.floor(local_y / CHUNK_SIZE)
    selected = None
    for tag, payload in chunks(data):
        if tag != b"MCNK":
            continue
        if len(payload) < 128:
            raise ValueError("truncated MCNK header (need 128 bytes)")
        index_x, index_y = struct.unpack_from("<II", payload, 4)
        if index_x >= 16 or index_y >= 16:
            raise ValueError(f"invalid MCNK chunk indices ({index_x}, {index_y})")
        origin_x, origin_y, base = struct.unpack_from("<3f", payload, 104)
        expected_x = tile_x - index_y * CHUNK_SIZE
        expected_y = tile_y - index_x * CHUNK_SIZE
        # Header positions are f32; computed tile origins are double precision.
        if not (
            math.isfinite(origin_x)
            and math.isfinite(origin_y)
            and abs(origin_x - expected_x) <= 0.01
            and abs(origin_y - expected_y) <= 0.01
        ):
            raise ValueError(
                f"MCNK ({index_x}, {index_y}) origin mismatches tile/chunk"
            )
        if (index_x, index_y) == (wanted_x, wanted_y):
            if selected is not None:
                raise ValueError(f"duplicate MCNK chunk ({wanted_x}, {wanted_y})")
            selected = (payload, base, expected_x, expected_y)
    if selected is None:
        raise ValueError(f"missing MCNK chunk ({wanted_x}, {wanted_y})")
    payload, base, origin_x, origin_y = selected
    heights = None
    for tag, sub in chunks(payload[128:]):
        if tag != b"MCVT":
            continue
        if heights is not None or len(sub) != 145 * 4:
            raise ValueError("MCVT must contain exactly one set of 145 heights")
        heights = struct.unpack("<145f", sub)
    if heights is None:
        raise ValueError("selected MCNK missing MCVT heights")
    if not math.isfinite(base) or not all(math.isfinite(h) for h in heights):
        raise ValueError("MCNK base height and MCVT heights must be finite")
    row_f = (origin_x - world_x) / UNIT_SIZE
    col_f = (origin_y - world_y) / UNIT_SIZE
    row, col = min(math.floor(row_f), 7), min(math.floor(col_f), 7)
    return base + interpolate_height(heights, row, col, col_f - col, row_f - row)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("adt", type=Path, help="extracted root ADT")
    parser.add_argument("--tile-row", type=int, required=True)
    parser.add_argument("--tile-col", type=int, required=True)
    parser.add_argument("--world-x", type=float, required=True)
    parser.add_argument("--world-y", type=float, required=True)
    args = parser.parse_args()
    try:
        height = sample_adt_height(
            args.adt.read_bytes(),
            args.world_x,
            args.world_y,
            args.tile_row,
            args.tile_col,
        )
    except (OSError, ValueError) as error:
        parser.exit(1, f"terrain height: {error}\n")
    print(f"{height:.6f}")


if __name__ == "__main__":
    main()
