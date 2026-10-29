#!/usr/bin/env python3
"""Join frozen position requests to geometry results without inventing altitude."""
import argparse
import hashlib
import json
from collections import Counter, defaultdict
from datetime import datetime, timedelta, timezone
from pathlib import Path


def timestamp_value(value):
    stamp, offset = value.rsplit('-', 1) if '-' in value else value.rsplit('+', 1)
    sign = -1 if '-' in value else 1
    return datetime.strptime(stamp, '%m/%d/%Y %H:%M:%S.%f').replace(
        tzinfo=timezone(sign * timedelta(hours=int(offset))))


def prepare(runs, clusters, requests, resolved, packs=(), centroid_requests=(), centroid_resolved=()):
    if len(requests) != len(resolved):
        raise ValueError('floor response count does not match frozen requests')
    floor = {}
    for request, result in zip(requests, resolved):
        key = (request['file'], request['run_start'], request['guid'])
        if key in floor:
            raise ValueError(f'duplicate floor request {key}')
        floor[key] = (request, result)
    if len(centroid_requests) != len(centroid_resolved):
        raise ValueError('centroid floor response count does not match requests')
    centroid_floor = {}
    for request, result in zip(centroid_requests, centroid_resolved):
        index = request['cluster_index']
        if index in centroid_floor:
            raise ValueError(f'duplicate centroid floor request {index}')
        centroid_floor[index] = (request, result)
    outside = set()
    bosses = set()
    for run in runs:
        for unit in run['creatures'].values():
            if unit['seen_outside_encounter']:
                outside.add((run['map'], unit['npc_id']))
        for encounter in run['encounters']:
            bosses.update((run['map'], npc) for npc in encounter.get('known_boss_npc_ids', []))
    pack_by_unit = {}
    for pack in packs:
        for member in pack['members']:
            key = (pack['file'], pack['run_start'], member['guid'])
            pack_by_unit[key] = pack['npc_ids']
    records = []
    gaps = defaultdict(Counter)
    for index, cluster in enumerate(clusters):
        map_id, npc_id = cluster['map'], cluster['npc_id']
        if cluster['ambiguous_match'] and 'candidate_centroid' not in cluster:
            gaps[map_id]['AMBIGUOUS_SPAWN_IDENTITY'] += 1
            continue
        if any(observation['summoned'] for observation in cluster['observations']):
            gaps[map_id]['SUMMON_DESTINATION'] += 1
            continue
        if (map_id, npc_id) not in outside and (map_id, npc_id) not in bosses:
            gaps[map_id]['MID_ENCOUNTER_ONLY_ID'] += 1
            continue
        chosen = None
        observations = sorted(cluster['observations'], key=lambda item: timestamp_value(item['time']))
        if cluster['ambiguous_match']:
            centroid = cluster['candidate_centroid']
            pair = centroid_floor.get(index)
            if pair is not None:
                request, result = pair
                if (request['map'], request['x'], request['y'], request['ui_map_id']) != (
                        map_id, centroid['x'], centroid['y'], cluster['ui_map_id']):
                    raise ValueError(f'centroid floor request does not match cluster {index}')
                if 'gap' not in result and result.get('method') in ('single_candidate', 'neighbor_path', 'authored_neighbor_anchor'):
                    chosen = observations[0], result, centroid['x'], centroid['y']
        else:
            for observation in observations:
                key = (observation['file'], observation['run_start'], observation['guid'])
                request, result = floor[key]
                if (request['map'], request['x'], request['y'], request['ui_map_id']) != (
                        map_id, observation['x'], observation['y'], cluster['ui_map_id']):
                    raise ValueError(f'floor request does not match observation {key}')
                if 'gap' not in result and result.get('method') in ('single_candidate', 'neighbor_path', 'authored_neighbor_anchor'):
                    chosen = observation, result, observation['x'], observation['y']
                    break
        if chosen is None:
            gaps[map_id]['CENTROID_Z_UNRESOLVED' if cluster['ambiguous_match'] else 'POSITION_Z_UNRESOLVED'] += 1
            continue
        observation, result, x, y = chosen
        identity = sorted((o['file'], o['run_start'], o['guid']) for o in cluster['observations'])
        source_key = hashlib.sha256(json.dumps([map_id, npc_id, identity], separators=(',', ':')).encode()).hexdigest()
        records.append({'source_key': source_key, 'map': map_id, 'npc_id': npc_id,
            'x': x, 'y': y, 'z': result['z'],
            'position_method': 'candidate_centroid' if cluster['ambiguous_match'] else 'first_resolved_observation',
            'observed_x': observation['x'], 'observed_y': observation['y'],
            'candidate_spread_yards': cluster.get('candidate_spread_yards'),
            'candidate_observations': cluster.get('candidate_observations') if cluster['ambiguous_match'] else None,
            'facing': observation['facing'], 'level': observation['level'], 'z_method': result['method'],
            'floor_anchor_rows': result.get('anchor_rows'), 'assignment_margin_yards': cluster.get('assignment_margin_yards'),
            'observation_event': observation['event'], 'observation_time': observation['time'],
            'source_file': observation['file'], 'source_run_start': observation['run_start'],
            'source_guid': observation['guid'],
            'pack_npc_ids': pack_by_unit.get((observation['file'], observation['run_start'], observation['guid'])),
            'population_kind': 'likely_spawn', 'summoned': False, 'identity_ambiguous': cluster['ambiguous_match']})
    return records, {str(map_id): dict(counts) for map_id, counts in sorted(gaps.items())}


def load(path):
    with path.open(encoding='utf-8') as source:
        return json.load(source)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('census', 'clusters', 'requests', 'resolved', 'output', 'gaps'):
        parser.add_argument('--' + name, required=True, type=Path)
    parser.add_argument('--packs', type=Path)
    parser.add_argument('--centroid-requests', type=Path)
    parser.add_argument('--centroid-resolved', type=Path)
    args = parser.parse_args()
    if bool(args.centroid_requests) != bool(args.centroid_resolved):
        parser.error('--centroid-requests and --centroid-resolved must be used together')
    packs = load(args.packs)['packs'] if args.packs else ()
    centroid_requests = load(args.centroid_requests) if args.centroid_requests else ()
    centroid_resolved = load(args.centroid_resolved) if args.centroid_resolved else ()
    records, gaps = prepare(load(args.census)['runs'], load(args.clusters), load(args.requests), load(args.resolved), packs, centroid_requests, centroid_resolved)
    args.output.write_text(json.dumps(records, indent=2) + '\n')
    args.gaps.write_text(json.dumps(gaps, indent=2) + '\n')


if __name__ == '__main__':
    main()
