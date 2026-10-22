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
    if mapper == []:
        mapper = {}  # Wowhead serializes a genuine empty mapper as an empty JSON array.
    if not isinstance(mapper, dict):
        raise ValueError(f'NPC{entry} has an invalid mapper object')
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


def measure_assignment_spread_yards(assignments):
    keys = ('UiMin_0', 'UiMin_1', 'UiMax_0', 'UiMax_1', 'Region_0', 'Region_1', 'Region_3', 'Region_4')
    values = {key: [float(a[key]) for a in assignments] for key in keys}
    if not all(math.isfinite(v) for column in values.values() for v in column):
        raise ValueError('journal assignments have nonfinite horizontal bounds')
    widths = [[float(a[f'UiMax_{i}']) - float(a[f'UiMin_{i}']) for i in (0, 1)] for a in assignments]
    if any(width <= 0 for pair in widths for width in pair):
        raise ValueError('journal assignments have degenerate UI bounds')
    u_scale = max(abs(float(a['Region_4']) - float(a['Region_1'])) / w[0] for a, w in zip(assignments, widths))
    v_scale = max(abs(float(a['Region_3']) - float(a['Region_0'])) / w[1] for a, w in zip(assignments, widths))
    spreads = {key: max(column) - min(column) for key, column in values.items()}
    for axis, scale in ((0, u_scale), (1, v_scale)):
        for edge in ('UiMin', 'UiMax'):
            spreads[f'{edge}_{axis}'] *= scale
    return spreads


def resolve_journal_assignments(assignments, marker):
    if not assignments:
        raise ValueError('journal UiMap has no assignment on the requested map')
    spread = measure_assignment_spread_yards(assignments)
    if any(value > 0.001 for value in spread.values()):
        raise ValueError('journal UiMap horizontal bounds are not equivalent within0.001yd')
    chosen = min(assignments, key=lambda a: int(a['OrderIndex']))
    points = [transform_marker(marker, [a]) for a in assignments]
    projected = {axis: max(p[i] for p in points) - min(p[i] for p in points) for i, axis in enumerate(('x', 'y'))}
    proof = dict(policy='FOLLOWER_PLACEMENT_EQUIVALENT_UIMAP_ASSIGNMENTS', bounds_tolerance_yards=0.001,
                 component_spread_yards=spread, projected_axis_spread_yards=projected,
                 chosen_assignment_id=int(chosen['ID']), chosen_order_index=int(chosen['OrderIndex']),
                 order_source='TC a352b1fa DB2Stores.cpp:3154-3157; exact lowest-OrderIndex row, no averaging')
    return [chosen] + [a for a in assignments if a is not chosen], proof


def read_journal_marker(tables, map_id, entry, journal_id):
    rows = [r for r in tables['JournalEncounter'] if int(r['ID']) == journal_id]
    if len(rows) != 1:
        raise ValueError('placement requires one authenticated journal encounter row')
    row = rows[0]
    assignments = [a for a in tables['UiMapAssignment']
                   if a['UiMapID'] == row['UiMapID'] and int(a['MapID']) == map_id]
    floors = {int(g['FloorIndex']) for g in tables['UiMapGroupMember'] if g['UiMapID'] == row['UiMapID']}
    if len(floors) > 1:
        raise ValueError('journal UiMap has ambiguous floor identities')
    percent = [float(row['Map_0']) * 100, float(row['Map_1']) * 100]
    if not all(math.isfinite(v) and 0 <= v <= 100 for v in percent):
        raise ValueError('journal marker is outside its UiMap')
    marker = dict(ui_map_id=int(row['UiMapID']), floor=next(iter(floors)) if floors else None, percent=percent)
    assignments, resolution = resolve_journal_assignments(assignments, marker)
    page = dict(entry=entry, source_table='JournalEncounter', source_row=row, assignment_resolution=resolution,
                source_row_sha256=hashlib.sha256(json.dumps(row, sort_keys=True).encode()).hexdigest(),
                url=f'db2:{BUILD}/JournalEncounter#{journal_id}', markers=[marker])
    return page, marker, assignments


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


def match_upward_source_floor(evidence, marker, assignments, heights):
    x, y = transform_marker(marker, assignments)
    upward = []
    for triangle in evidence['triangles']:
        a, b, c = triangle['world_vertices']
        normal_z = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
        if normal_z <= 0:
            continue
        bevy = [(v[0], v[2], -v[1]) for v in (a, b, c)]
        height = triangle_height((x, -y), bevy)
        if height is not None:
            upward.append(height)
    if not upward:
        raise ValueError('source floor group has no upward triangle at source XY')
    if evidence['policy'] == 'FOLLOWER_PLACEMENT_Z_FROM_SOURCE_FLOOR_GROUP_UPWARD_TRIANGLE' and len(upward) != 1:
        raise ValueError('source floor group requires exactly one upward triangle at source XY')
    tolerance = evidence['maximum_height_difference_yards']
    if not math.isfinite(tolerance) or tolerance <= 0:
        raise ValueError('source floor matching tolerance must be finite and positive')
    matched = [z for z in heights if all(abs(z - height) <= tolerance for height in upward)]
    if len(matched) != 1:
        raise ValueError('source upward floor requires exactly one baked candidate within tolerance')
    return matched


def read_source_floor_support(record, marker, assignments, heights):
    evidence = record.get('source_floor_evidence')
    triangle_policy = (isinstance(evidence, dict) and evidence.get('policy') in (
        'FOLLOWER_PLACEMENT_Z_FROM_SOURCE_FLOOR_GROUP_UPWARD_TRIANGLE',
        'FOLLOWER_PLACEMENT_Z_FROM_SOURCE_FLOOR_UPWARD_CONSENSUS'))
    if 'source_floor_z_bounds' not in record and not triangle_policy:
        return heights
    if marker['floor'] != record['source_floor']:
        raise ValueError('source WMO floor differs from the cached marker floor')
    matching = [a for a in assignments if int(a['WMOGroupID']) == evidence['wmo_group_id']
                and int(a['WMODoodadPlacementID']) == evidence['placement_unique_id']]
    if not matching:
        raise ValueError('source WMO group/placement is not assigned to this DB2 floor')
    for artifact in evidence['source_files']:
        if digest(artifact['path']) != artifact['sha256']:
            raise ValueError('source WMO floor evidence file changed')
    if triangle_policy:
        return match_upward_source_floor(evidence, marker, assignments, heights)
    if evidence['rotation_degrees'][0] != 0 or evidence['rotation_degrees'][2] != 0:
        raise ValueError('floor bounds require an explicit vertical transform for tilted placements')
    bounds = [evidence['placement_position_z'] + z * evidence['scale'] for z in evidence['group_local_z']]
    if bounds != record['source_floor_z_bounds'] or not all(math.isfinite(z) for z in bounds) or bounds[0] >= bounds[1]:
        raise ValueError('source floor bounds differ from the recorded WMO transform')
    return [z for z in heights if bounds[0] <= z <= bounds[1]]


def sample_surfaces(polygons, x, y, assignments):
    heights = set()
    for polygon in polygons:
        for index in range(1, len(polygon) - 1):
            height = triangle_height((x, -y), (polygon[0], polygon[index], polygon[index + 1]))
            if height is not None and any(float(a["Region_2"]) <= height <= float(a["Region_5"]) for a in assignments):
                heights.add(height)
    return sorted(heights)
