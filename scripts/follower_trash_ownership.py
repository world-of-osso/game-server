"""Prove previous-packet ownership before removing only excess Blinding spawns."""
from contextlib import closing
import json
from pathlib import Path

from follower_placement_sources import digest
from follower_placements import PLACEMENT_POLICY, open_readonly
from follower_trash_density import marker_key


def read_owned_rows(db, lower, upper):
    cursor = db.execute('SELECT * FROM content_creature WHERE guid BETWEEN ? AND ? ORDER BY guid', (lower, upper))
    columns = [column[0] for column in cursor.description]
    return {row[0]: dict(zip(columns, row)) for row in cursor}


def read_previous_packet(evidence, map_id):
    if map_id != 2859:
        raise ValueError('only the explicit Blinding owned-spawn correction is authorized')
    path = Path(evidence['receipt']).resolve()
    if digest(path) != evidence['receipt_sha256']:
        raise ValueError('previous receipt changed')
    receipt = json.loads(path.read_text())
    if receipt['map_id'] != map_id or receipt['policy'] != PLACEMENT_POLICY or receipt['guid_range'] != [63859100, 63859999]:
        raise ValueError('previous packet does not prove this map and owned namespace')
    if digest(path.parent / 'delta.sql') != receipt['sql_sha256'] or digest(path.parent / 'replayed.sqlite') != receipt['replayed_sha256']:
        raise ValueError('previous SQL/replayed ownership proof changed')
    return path, receipt


def load_owned_spawns(baseline, map_id, qualified, evidence):
    path, reference = read_previous_packet(evidence, map_id)
    lower, upper = reference['guid_range']
    points = {marker_key(point): point for point in reference['placements']}
    if len(points) != len(reference['placements']):
        raise ValueError('previous packet has duplicate source marker identities')
    if any(not lower <= point['guid'] <= upper for point in points.values()):
        raise ValueError('previous actor lies outside the owned namespace')
    with closing(open_readonly(path.parent / 'replayed.sqlite')) as source:
        expected = read_owned_rows(source, lower, upper)
    if set(expected) != {point['guid'] for point in points.values()}:
        raise ValueError('reference namespace contains unowned or missing rows')
    actual = read_owned_rows(baseline, lower, upper)
    if actual != expected:
        raise ValueError('owned baseline rows changed or namespace contains another owner')
    if {marker_key(point) for point in qualified} != set(points):
        raise ValueError('qualified source footprint differs from published actors')
    for point in qualified:
        original = points[marker_key(point)]
        row = expected[original['guid']]
        if row['id1'] != point['entry'] or row['map'] != map_id or [row['position_x'], row['position_y'], row['position_z']] != point['wow_xyz']:
            raise ValueError('source pose or identity changed; correction cannot relocate an actor')
    owned = {key: expected[point['guid']] for key, point in points.items()}
    proof = dict(reference_receipt=str(path), reference_receipt_sha256=evidence['receipt_sha256'],
                 reference_sql_sha256=reference['sql_sha256'], reference_replayed_sha256=reference['replayed_sha256'],
                 guid_range=[lower, upper], owned_spawn_count=len(owned),
                 guard='every complete current owned row equals the pinned published row; no new or foreign namespace rows')
    return owned, proof
