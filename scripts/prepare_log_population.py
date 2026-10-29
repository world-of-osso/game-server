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


def prepare(runs, clusters, requests, resolved):
    if len(requests) != len(resolved):
        raise ValueError('floor response count does not match frozen requests')
    floor = {}
    for request, result in zip(requests, resolved):
        key = (request['file'], request['run_start'], request['guid'])
        if key in floor:
            raise ValueError(f'duplicate floor request {key}')
        floor[key] = (request, result)
    outside = set()
    bosses = set()
    for run in runs:
        for unit in run['creatures'].values():
            if unit['seen_outside_encounter']:
                outside.add((run['map'], unit['npc_id']))
        for encounter in run['encounters']:
            bosses.update((run['map'], npc) for npc in encounter.get('known_boss_npc_ids', []))
    records = []
    gaps = defaultdict(Counter)
    for cluster in clusters:
        map_id, npc_id = cluster['map'], cluster['npc_id']
        if cluster['ambiguous_match']:
            gaps[map_id]['AMBIGUOUS_SPAWN_IDENTITY'] += 1
            continue
        if any(observation['summoned'] for observation in cluster['observations']):
            gaps[map_id]['SUMMON_DESTINATION'] += 1
            continue
        if (map_id, npc_id) not in outside and (map_id, npc_id) not in bosses:
            gaps[map_id]['MID_ENCOUNTER_ONLY_ID'] += 1
            continue
        chosen = None
        for observation in sorted(cluster['observations'], key=lambda item: timestamp_value(item['time'])):
            key = (observation['file'], observation['run_start'], observation['guid'])
            request, result = floor[key]
            if (request['map'], request['x'], request['y'], request['ui_map_id']) != (
                    map_id, observation['x'], observation['y'], cluster['ui_map_id']):
                raise ValueError(f'floor request does not match observation {key}')
            if 'gap' not in result and result.get('method') in ('single_candidate', 'neighbor_path'):
                chosen = observation, result
                break
        if chosen is None:
            gaps[map_id]['POSITION_Z_UNRESOLVED'] += 1
            continue
        observation, result = chosen
        identity = sorted((o['file'], o['run_start'], o['guid']) for o in cluster['observations'])
        source_key = hashlib.sha256(json.dumps([map_id, npc_id, identity], separators=(',', ':')).encode()).hexdigest()
        records.append({'source_key': source_key, 'map': map_id, 'npc_id': npc_id,
            'x': observation['x'], 'y': observation['y'], 'z': result['z'],
            'facing': observation['facing'], 'level': observation['level'], 'z_method': result['method'],
            'observation_event': observation['event'], 'observation_time': observation['time'],
            'population_kind': 'likely_spawn', 'summoned': False, 'identity_ambiguous': False})
    return records, {str(map_id): dict(counts) for map_id, counts in sorted(gaps.items())}


def load(path):
    with path.open(encoding='utf-8') as source:
        return json.load(source)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('census', 'clusters', 'requests', 'resolved', 'output', 'gaps'):
        parser.add_argument('--' + name, required=True, type=Path)
    args = parser.parse_args()
    records, gaps = prepare(load(args.census)['runs'], load(args.clusters), load(args.requests), load(args.resolved))
    args.output.write_text(json.dumps(records, indent=2) + '\n')
    args.gaps.write_text(json.dumps(gaps, indent=2) + '\n')


if __name__ == '__main__':
    main()
