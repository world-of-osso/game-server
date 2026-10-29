#!/usr/bin/env python3
"""Extract Ruby encounter observations from user-approved combat logs.

Run with python3 -I scripts/ruby_log_extract.py LOG [LOG ...]. Input is data,
never code. Only successful encounter windows and listed boss casts are used.
Positions are first observed in-combat samples, not spawn coordinates or floors.
"""
import argparse
import bisect
import collections
import csv
import datetime
import json
import statistics
from pathlib import Path

CASTS = {
    (2606, 189232, 372863, 'SPELL_CAST_START'),
    (2606, 189232, 372107, 'SPELL_CAST_START'),
    (2606, 189232, 372858, 'SPELL_CAST_SUCCESS'),
    (2623, 190484, 381525, 'SPELL_CAST_START'),
    (2623, 190484, 381602, 'SPELL_CAST_START'),
    (2623, 190484, 381605, 'SPELL_CAST_START'),
    (2623, 190485, 381512, 'SPELL_CAST_START'),
    (2623, 190485, 381517, 'SPELL_CAST_START'),
    (2623, 190485, 381516, 'SPELL_CAST_START'),
}


def entry(guid):
    parts = guid.split('-')
    return int(parts[-2]) if len(parts) > 5 and parts[-2].isdigit() else None


def summarize(values):
    return {'median': statistics.median(values), 'min': min(values), 'max': max(values), 'count': len(values)} if values else None


def extract(paths):
    kills = []
    for path in paths:
        active = None
        with Path(path).open(encoding='utf-8', errors='replace') as stream:
            for line in stream:
                try:
                    timestamp, raw = line.split('  ', 1)
                    time = datetime.datetime.strptime(timestamp.rsplit('-', 1)[0], '%m/%d/%Y %H:%M:%S.%f')
                    row = next(csv.reader([raw]))
                    event = row[0]
                    if event == 'ENCOUNTER_START' and row[1] in ('2606', '2623'):
                        active = {'encounter': int(row[1]), 'start': time, 'casts': collections.defaultdict(list), 'positions': {}, 'health_50': {}, 'stage_event': None, 'energy': [], 'energizes': [], 'self_auras': set()}
                        continue
                    if event == 'ENCOUNTER_END' and active and int(row[1]) == active['encounter']:
                        if row[5] == '1':
                            kills.append(active)
                        active = None
                        continue
                    if active is None or len(row) < 11:
                        continue
                    source = entry(row[1])
                    if row[9].isdigit():
                        key = (active['encounter'], source, int(row[9]), event)
                        if key in CASTS:
                            active['casts'][key].append((time - active['start']).total_seconds())
                    if active['encounter'] == 2606:
                        elapsed = (time - active['start']).total_seconds()
                        if entry(row[5]) == 189232 and event in ('SPELL_ENERGIZE', 'SPELL_PERIODIC_ENERGIZE'):
                            active['energizes'].append([elapsed, int(row[9])])
                        if entry(row[1]) == entry(row[5]) == 189232 and event.startswith('SPELL_AURA_'):
                            active['self_auras'].add((event, row[9]))
                        for index in (9, 12):
                            if len(row) > index + 17 and entry(row[index]) == 189232 and row[index + 10] == '3' and row[index + 11].isdigit():
                                active['energy'].append((elapsed, int(row[index + 11])))
                    if active['encounter'] == 2623:
                        elapsed = (time - active['start']).total_seconds()
                        if event == 'SPELL_AURA_APPLIED' and row[9] == '181089' and entry(row[5]) == 190485:
                            active['stage_event'] = elapsed
                        for index in (9, 12):
                            if len(row) <= index + 17:
                                continue
                            actor = entry(row[index])
                            if actor not in (190484, 190485):
                                continue
                            if not row[index + 2].isdigit() or not row[index + 3].isdigit():
                                continue
                            health, maximum = int(row[index + 2]), int(row[index + 3])
                            if maximum and health * 2 <= maximum:
                                active['health_50'].setdefault(actor, elapsed)
                            if actor not in active['positions']:
                                x, y, map_id, orientation = row[index + 14:index + 18]
                                active['positions'][actor] = {
                                    'x': float(x), 'y': float(y),
                                    'logged_map_id': int(map_id), 'orientation': float(orientation),
                                }
                except (ValueError, IndexError, csv.Error):
                    continue
    result = {'kills': collections.Counter(str(kill['encounter']) for kill in kills), 'timers': {}, 'positions': {}, 'stage_events': [], 'kokia_energy': []}
    for kill in kills:
        if kill['encounter'] != 2606:
            continue
        samples = kill['energy']
        times = [sample[0] for sample in samples]
        rates = []
        for time, energy in samples:
            if not 5 <= energy <= 65:
                continue
            index = bisect.bisect_left(times, time + 8)
            if index < len(samples):
                end, value = samples[index]
                if end - time <= 8.5 and energy < value < 95:
                    rates.append((value - energy) / (end - time))
        result['kokia_energy'].append({'sample_count': len(samples), 'rate_8s': summarize(rates), 'energizes': kill['energizes'], 'self_auras': sorted(kill['self_auras'])})
    for kill in kills:
        if kill['encounter'] == 2623:
            result['stage_events'].append({'first_observed_health_50': kill['health_50'], 'aura_181089': kill['stage_event']})
    for key in sorted(CASTS):
        sequences = [kill['casts'][key] for kill in kills if key in kill['casts']]
        first = [sequence[0] for sequence in sequences]
        repeats = [b - a for sequence in sequences for a, b in zip(sequence, sequence[1:])]
        result['timers'][f'{key[1]}:{key[2]}:{key[3]}'] = {'first': summarize(first), 'repeat': summarize(repeats)}
    for actor in (190484, 190485):
        result['positions'][str(actor)] = [kill['positions'][actor] for kill in kills if actor in kill['positions']]
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('logs', nargs='+', type=Path)
    args = parser.parse_args()
    print(json.dumps(extract(args.logs), indent=2, sort_keys=True))


if __name__ == '__main__':
    main()
