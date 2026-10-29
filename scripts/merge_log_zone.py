#!/usr/bin/env python3
"""Merge exact same-file GUIDs from zone-entry intervals into key observations."""
import argparse
import copy
import importlib.util
import json
from collections import Counter, defaultdict
from pathlib import Path

spec = importlib.util.spec_from_file_location('log_population_census', Path(__file__).with_name('log_population_census.py'))
census = importlib.util.module_from_spec(spec)
spec.loader.exec_module(census)


def merge(key_runs, zone_runs):
    merged = copy.deepcopy(key_runs)
    index = defaultdict(list)
    for run in merged:
        for guid, unit in run['creatures'].items():
            index[(run['file'], run['map'], guid)].append((run, unit))
    counts = defaultdict(Counter)
    for zone in copy.deepcopy(zone_runs):
        remainder = {}
        for guid, unit in zone['creatures'].items():
            key = (zone['file'], zone['map'], guid)
            options = index.get(key, [])
            if not options:
                remainder[guid] = unit
                counts[zone['map']]['zone_only_guids'] += 1
                continue
            observed = census.timestamp_value(unit['first_seen'])
            distances = [(abs((census.timestamp_value(item['first_seen']) - observed).total_seconds()), run, item)
                         for run, item in options]
            distances.sort(key=lambda value: value[0])
            if len(distances) > 1 and distances[0][0] == distances[1][0]:
                remainder[guid] = unit
                counts[zone['map']]['ambiguous_episode'] += 1
                continue
            _, run, target = distances[0]
            counts[zone['map']]['matched_exact_guid'] += 1
            if observed < census.timestamp_value(target['first_seen']):
                target['first_seen'] = unit['first_seen']
                target['first_event'] = unit['first_event']
                target['first_encounter_id'] = unit['first_encounter_id']
            position = unit.get('first_position')
            current = target.get('first_position')
            if position and (not current or census.timestamp_value(position['time']) < census.timestamp_value(current['time'])):
                target['first_position'] = position
                target['position_neighbors'] = unit.get('position_neighbors', [])
                target['position_source_run_start'] = zone['start']
                counts[zone['map']]['earlier_position'] += 1
            for field in ('summoned', 'player_controlled', 'control_flags_invalid', 'seen_outside_encounter'):
                target[field] |= unit[field]
        if remainder:
            zone['creatures'] = remainder
            merged.append(zone)
    return merged, {str(map_id): dict(values) for map_id, values in sorted(counts.items())}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for field in ('keys', 'zones', 'output', 'report'):
        parser.add_argument('--' + field, required=True, type=Path)
    args = parser.parse_args()
    with args.keys.open(encoding='utf-8') as source:
        keys = json.load(source)['runs']
    with args.zones.open(encoding='utf-8') as source:
        zones = json.load(source)['runs']
    runs, report = merge(keys, zones)
    args.output.write_text(json.dumps({'runs': runs}, indent=2) + '\n')
    args.report.write_text(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
