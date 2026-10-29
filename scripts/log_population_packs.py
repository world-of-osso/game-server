#!/usr/bin/env python3
"""Build provisional cross-run pack matches from pre-engagement position samples."""
import argparse
import json
from datetime import datetime, timedelta, timezone
from collections import Counter, defaultdict
from pathlib import Path


def build_packs(samples, radius=5.0, seconds=3.0):
    by_run = defaultdict(dict)
    for sample in samples:
        run = (sample['file'], sample['run_start'], sample['map'])
        existing = by_run[run].get(sample['guid'])
        if existing is None or sample['seconds_from_start'] < existing['seconds_from_start']:
            by_run[run][sample['guid']] = sample
    packs = []
    for run, units in sorted(by_run.items()):
        pending = list(sorted(units.values(), key=lambda unit: unit['guid']))
        while pending:
            members = [pending.pop(0)]
            i = 0
            while i < len(members):
                current = members[i]
                nearby = [other for other in pending if other['ui_map_id'] == current['ui_map_id']
                          and abs(other['seconds_from_start'] - current['seconds_from_start']) <= seconds
                          and (other['x'] - current['x']) ** 2 + (other['y'] - current['y']) ** 2 <= radius ** 2]
                for other in nearby:
                    pending.remove(other)
                    members.append(other)
                i += 1
            if len(members) < 2:
                continue
            ids = sorted(int(member['guid'].split('-')[5]) for member in members)
            packs.append({'file': run[0], 'run_start': run[1], 'map': run[2],
                          'ui_map_id': members[0]['ui_map_id'], 'npc_ids': ids,
                          'members': members})
    return packs


def timestamp_value(value):
    stamp, offset = value.rsplit('-', 1) if '-' in value else value.rsplit('+', 1)
    sign = -1 if '-' in value else 1
    return datetime.strptime(stamp, '%m/%d/%Y %H:%M:%S.%f').replace(
        tzinfo=timezone(sign * timedelta(hours=int(offset))))


def filter_pack_activity(packs, runs, margin_seconds=3.0):
    units = {(run['file'], run['start'], guid): unit
             for run in runs for guid, unit in run['creatures'].items()}
    eligible = []
    for pack in packs:
        boundaries = []
        for member in pack['members']:
            unit = units[(pack['file'], pack['run_start'], member['guid'])]
            boundaries.extend(timestamp_value(unit[key]) for key in ('first_hostile_time', 'first_cast_time') if key in unit)
        if boundaries and any((min(boundaries) - timestamp_value(member['time'])).total_seconds() < margin_seconds
                              for member in pack['members']):
            continue
        eligible.append(pack)
    return eligible


def match_packs(packs, radius=60.0):
    groups = defaultdict(list)
    for pack in packs:
        groups[(pack['map'], pack['ui_map_id'], tuple(pack['npc_ids']))].append(pack)
    candidates = []
    for (map_id, ui_map_id, ids), observations in sorted(groups.items()):
        run_counts = Counter((p['file'], p['run_start']) for p in observations)
        unique = [p for p in observations if run_counts[(p['file'], p['run_start'])] == 1]
        for pack in unique:
            members = pack['members']
            pack['center'] = [sum(m['x'] for m in members) / len(members),
                              sum(m['y'] for m in members) / len(members)]
        # Uniqueness per run is necessary, not sufficient, to prove pack identity.
        possible = []
        for pack in unique:
            if not possible:
                possible.append(pack)
                continue
            anchor = possible[0]['center']
            if sum((a - b) ** 2 for a, b in zip(anchor, pack['center'])) <= radius ** 2:
                possible.append(pack)
        if len(possible) >= 2:
            candidates.append({'map': map_id, 'ui_map_id': ui_map_id, 'npc_ids': ids,
                               'observations': possible,
                               'gap': 'PACK_IDENTITY_NAVMESH_AND_ENGAGEMENT_UNVERIFIED'})
    return candidates


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--samples', required=True, type=Path)
    parser.add_argument('--census', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    with args.samples.open(encoding='utf-8') as source:
        packs = build_packs(json.loads(line) for line in source)
    with args.census.open(encoding='utf-8') as source:
        packs = filter_pack_activity(packs, json.load(source)['runs'])
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps({'packs': packs, 'cross_run_candidates': match_packs(packs)}, indent=2) + '\n')


if __name__ == '__main__':
    main()
