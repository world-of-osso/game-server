"""Read cached Wowhead markers and authenticated DB2; never fetch or invent Z."""
import csv
import hashlib
import json
import math
from pathlib import Path
import re

from wowhead_forever import percent_to_world

BUILD = "12.1.5.70448"
TABLES = ("AreaTable", "UiMap", "UiMapAssignment", "UiMapGroupMember")


def digest(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def read_db2(directory):
    directory = Path(directory)
    receipt = json.loads((directory / "receipt.json").read_text())
    if receipt["identity"]["build"] != BUILD or receipt["identity"]["extraction_source"] != "authenticated-local-CASC-content-root":
        raise ValueError("require authenticated local CASC build70448")
    tables, proof = {}, {}
    for table in TABLES:
        path = directory / f"{table}.csv"
        metadata = receipt["tables"][table]
        if not metadata["content_authenticated"] or metadata["unreadable_records"] or digest(path) != metadata["csv_sha256"]:
            raise ValueError(f"{table} authentication/hash mismatch")
        with path.open(newline="", encoding="utf-8") as stream:
            tables[table] = list(csv.DictReader(stream))
        proof[table] = metadata
    return tables, proof


def read_json_payload(source, pattern):
    match = re.search(pattern, source)
    if match is None:
        raise ValueError(f"cached NPC page lacks {pattern}")
    return json.JSONDecoder().raw_decode(source[match.end():])[0]


def read_cached_npc(cache, entry):
    url = f"https://www.wowhead.com/npc={entry}"
    key = hashlib.sha256(url.encode()).hexdigest()
    body = Path(cache) / f"{key}.html"
    metadata = json.loads(body.with_suffix(".json").read_text())
    if metadata["url"] != url or metadata["status"] != 200:
        raise ValueError(f"NPC{entry} cache metadata mismatch")
    raw = body.read_text()
    source = json.loads(raw) if raw.startswith('"') else raw
    entity = read_json_payload(source, rf"\$\.extend\(g_npcs\[{entry}\],\s*")
    if entity["id"] != entry:
        raise ValueError(f"NPC{entry} page identity mismatch")
    mapper = read_json_payload(source, r"g_mapperData\s*=\s*")
    markers = []
    for zone, levels in mapper.items():
        indexed = enumerate(levels) if isinstance(levels, list) else levels.items()
        for floor, data in indexed:
            for point in data["coords"]:
                if len(point) != 2 or not all(math.isfinite(v) and 0 <= v <= 100 for v in point):
                    raise ValueError(f"NPC{entry} invalid marker")
                markers.append(dict(zone=int(zone), floor=int(floor), percent=point))
    return dict(entry=entry, entity=entity, markers=markers, url=url,
                cache_body=str(body), cache_sha256=digest(body), cache_metadata=metadata)


def select_assignment(tables, map_id, marker):
    area = next(row for row in tables["AreaTable"] if int(row["ID"]) == marker["zone"])
    if int(area["ContinentID"]) != map_id:
        raise ValueError("Wowhead zone does not belong to requested DB2 map")
    assignments = [row for row in tables["UiMapAssignment"] if int(row["MapID"]) == map_id]
    map_ids = {row["UiMapID"] for row in assignments}
    floors = {row["UiMapID"] for row in tables["UiMapGroupMember"]
              if row["UiMapID"] in map_ids and int(row["FloorIndex"]) == marker["floor"]}
    if len(floors) != 1:
        raise ValueError(f"map{map_id} floor{marker['floor']} lacks a unique DB2 floor identity")
    candidates = [row for row in assignments if row["UiMapID"] in floors]
    bounds = {tuple(row[key] for key in ("UiMin_0", "UiMin_1", "UiMax_0", "UiMax_1",
                                        "Region_0", "Region_1", "Region_3", "Region_4")) for row in candidates}
    if len(bounds) != 1:
        raise ValueError("floor has multiple horizontal bounds; cannot guess assignment")
    return candidates


def transform_marker(marker, assignments):
    row = assignments[0]
    bounds = dict(ui_min=[float(row[f"UiMin_{i}"]) for i in range(2)],
                  ui_max=[float(row[f"UiMax_{i}"]) for i in range(2)],
                  region_min=[float(row[f"Region_{i}"]) for i in range(3)],
                  region_max=[float(row[f"Region_{i+3}"]) for i in range(3)])
    return percent_to_world(marker["percent"], bounds)


def read_surface_polygons(path):
    with Path(path).open() as stream:
        return [[tuple(map(float, vertex.split(','))) for vertex in line.strip().split(';')]
                for line in stream if line.strip()]


def triangle_height(point, triangle):
    # Bevy x,z span the plane; Bevy y is WoW height. Nav polygons are convex.
    x, z = point
    a, b, c = triangle
    denominator = (b[2] - c[2]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[2] - c[2])
    if denominator == 0:
        return None
    wa = ((b[2] - c[2]) * (x - c[0]) + (c[0] - b[0]) * (z - c[2])) / denominator
    wb = ((c[2] - a[2]) * (x - c[0]) + (a[0] - c[0]) * (z - c[2])) / denominator
    wc = 1 - wa - wb
    if min(wa, wb, wc) < 0:
        return None
    return wa * a[1] + wb * b[1] + wc * c[1]


def sample_surfaces(polygons, x, y, assignments):
    heights = set()
    for polygon in polygons:
        for index in range(1, len(polygon) - 1):
            height = triangle_height((x, -y), (polygon[0], polygon[index], polygon[index + 1]))
            if height is not None and any(float(a["Region_2"]) <= height <= float(a["Region_5"]) for a in assignments):
                heights.add(height)
    return sorted(heights)
