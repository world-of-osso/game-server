#!/usr/bin/env python3
"""Stream combat logs into a per-run Creature GUID census; input is untrusted data."""
import argparse
import csv
import json
import math
import re
import sqlite3
from collections import defaultdict
from datetime import datetime, timedelta, timezone
from pathlib import Path


def advanced_position(fields):
    """Return only a block that identifies its own source or destination unit."""
    if len(fields) < 28:
        return None
    for start in range(9, len(fields) - 18):
        if fields[start] not in (fields[1], fields[5]):
            continue
        try:
            int(fields[start + 2])  # current health begins the advanced block
            int(fields[start + 3])
            return {
                "guid": fields[start],
                "x": float(fields[start + 14]),
                "y": float(fields[start + 15]),
                "ui_map_id": int(fields[start + 16]),
                "facing": float(fields[start + 17]),
                "level": int(fields[start + 18]),
            }
        except ValueError:
            continue
    return None


def timestamp_value(value):
    stamp, offset = value.rsplit('-', 1) if '-' in value else value.rsplit('+', 1)
    sign = -1 if '-' in value else 1
    tz = timezone(sign * timedelta(hours=int(offset)))
    return datetime.strptime(stamp, '%m/%d/%Y %H:%M:%S.%f').replace(tzinfo=tz)


def engagement_guids(fields):
    event = fields[0]
    offensive = (event.endswith('_DAMAGE') or event.endswith('_DAMAGE_LANDED')
                 or event.endswith('_MISSED') or event in ('SPELL_CAST_START', 'SPELL_CAST_SUCCESS'))
    offensive |= event == 'SPELL_AURA_APPLIED' and len(fields) > 12 and fields[12] == 'DEBUFF'
    if not offensive:
        return ()
    try:
        reactions = [int(fields[3], 0) & 0xf0, int(fields[7], 0) & 0xf0]
    except ValueError:
        return ()
    if set(reactions) != {0x10, 0x40}:
        return ()
    return tuple(guid for guid in (fields[1], fields[5]) if guid.startswith('Creature-'))


def census(paths, maps, samples=None, zones_only=False):
    runs = []
    for path in sorted(paths):
        current = None
        encounter = None
        engaged = set()
        activity = {}
        in_key = False
        with path.open(encoding="utf-8", errors="replace") as source:
            for line in source:
                timestamp, sep, payload = line.partition("  ")
                if not sep:
                    continue
                event = payload.split(",", 1)[0]
                if event == "CHALLENGE_MODE_START":
                    fields = next(csv.reader([payload]))
                    current = None
                    in_key = True
                    if zones_only:
                        continue
                    encounter = None
                    engaged = set()
                    activity = {}
                    if len(fields) > 3 and fields[2].isdigit() and int(fields[2]) in maps:
                        current = {"file": path.name, "map": int(fields[2]), "start": timestamp,
                                   "name": fields[1], "challenge_id": int(fields[3]), "creatures": {},
                                   "encounters": []}
                        runs.append(current)
                    continue
                if event == "CHALLENGE_MODE_END":
                    current = None
                    encounter = None
                    in_key = False
                    continue
                if zones_only and event == 'ZONE_CHANGE' and not in_key:
                    fields = next(csv.reader([payload]))
                    current = None
                    encounter = None
                    engaged = set()
                    activity = {}
                    if len(fields) > 2 and fields[1].isdigit() and int(fields[1]) in maps:
                        current = {'file': path.name, 'map': int(fields[1]), 'start': timestamp,
                                   'name': fields[2], 'challenge_id': None, 'session_kind': 'zone',
                                   'creatures': {}, 'encounters': []}
                        runs.append(current)
                    continue
                if current is None:
                    continue
                if event == 'ENCOUNTER_START':
                    fields = next(csv.reader([payload]))
                    encounter = {'id': int(fields[1]), 'name': fields[2], 'start': timestamp,
                                 'units': {}, 'casts': [], 'phase_markers': []}
                    current['encounters'].append(encounter)
                    continue
                if event == 'ENCOUNTER_END':
                    fields = next(csv.reader([payload]))
                    if encounter is not None and int(fields[1]) == encounter['id']:
                        encounter['end'] = timestamp
                        encounter['success'] = fields[5] == '1'
                        encounter = None
                    continue
                if encounter is not None and 'PHASE' in event:
                    encounter['phase_markers'].append({'event': event, 'time': timestamp,
                                                       'fields': next(csv.reader([payload]))[1:]})
                if "Creature-" not in payload:
                    continue
                fields = next(csv.reader([payload]))
                if len(fields) < 9:
                    continue
                for guid in engagement_guids(fields):
                    engaged.add(guid)
                    activity.setdefault(guid, {}).setdefault('first_hostile_time', timestamp)
                if event in ('SPELL_CAST_START', 'SPELL_CAST_SUCCESS') and fields[1].startswith('Creature-'):
                    engaged.add(fields[1])
                    activity.setdefault(fields[1], {}).setdefault('first_cast_time', timestamp)
                for index in (1, 5):
                    guid = fields[index]
                    parts = guid.split("-")
                    if len(parts) != 7 or parts[0] != "Creature" or not parts[5].isdigit():
                        continue
                    creatures = current["creatures"]
                    item = creatures.setdefault(guid, {"npc_id": int(parts[5]), "spawn_uid": parts[6],
                                                       "instance": parts[4], "name": fields[index + 1],
                                                       "first_seen": timestamp, "first_event": event,
                                                       "first_encounter_id": encounter['id'] if encounter else None,
                                                       "seen_outside_encounter": False, "summoned": False,
                                                       "player_controlled": False, "control_flags_invalid": False})
                    try:
                        item['player_controlled'] |= bool(int(fields[index + 2], 0) & 0x100)
                    except ValueError:
                        item['control_flags_invalid'] = True
                    item.update(activity.get(guid, {}))
                    if encounter is None:
                        item['seen_outside_encounter'] = True
                    if encounter is not None:
                        encounter['units'][guid] = {'npc_id': int(parts[5]), 'name': fields[index + 1]}
                    if event == "SPELL_SUMMON" and index == 5:
                        item["summoned"] = True
                if encounter is not None and event in ('SPELL_CAST_START', 'SPELL_CAST_SUCCESS') and fields[1].startswith('Creature-') and len(fields) > 11:
                    encounter['casts'].append({'guid': fields[1], 'event': event, 'spell_id': int(fields[9]),
                        'name': fields[10], 'seconds_from_pull': (timestamp_value(timestamp) - timestamp_value(encounter['start'])).total_seconds()})
                position = advanced_position(fields)
                if position and position['guid'] in current['creatures']:
                    item = current['creatures'][position['guid']]
                    observation = {**position, 'time': timestamp, 'event': event}
                    if samples is not None and position['guid'] not in engaged:
                        elapsed = (timestamp_value(timestamp) - timestamp_value(current['start'])).total_seconds()
                        sample = {**observation, 'map': current['map'], 'file': current['file'],
                                  'run_start': current['start'], 'seconds_from_start': elapsed}
                        samples.write(json.dumps(sample, separators=(',', ':')) + '\n')
                    if 'first_position' not in item:
                        item['first_position'] = observation
                        item['position_neighbors'] = []
                    elif len(item['position_neighbors']) < 2 and (
                        observation['x'], observation['y'], observation['ui_map_id']
                    ) != (
                        item['first_position']['x'], item['first_position']['y'],
                        item['first_position']['ui_map_id']
                    ):
                        item['position_neighbors'].append(observation)
    return runs


def annotate_bosses(runs, database):
    with sqlite3.connect(f'file:{database}?mode=ro', uri=True) as conn:
        rows = conn.execute('SELECT map_id, encounter_id, creature_entry FROM raid_encounter_creature')
        bosses = {}
        for map_id, encounter_id, creature_entry in rows:
            bosses.setdefault((map_id, encounter_id), set()).add(creature_entry)
    for run in runs:
        for encounter in run['encounters']:
            known = bosses.get((run['map'], encounter['id']), set())
            encounter['known_boss_npc_ids'] = sorted(known)
            encounter['unclassified_npc_ids'] = sorted({u['npc_id'] for u in encounter['units'].values()} - known)
            for cast in encounter['casts']:
                cast['known_boss'] = encounter['units'][cast['guid']]['npc_id'] in known


def filter_safe_samples(runs, source, output, margin_seconds=3.0):
    activity = {(run['file'], run['start'], guid): unit
                for run in runs for guid, unit in run['creatures'].items()}
    for line in source:
        sample = json.loads(line)
        unit = activity[(sample['file'], sample['run_start'], sample['guid'])]
        boundaries = [timestamp_value(unit[key]) for key in ('first_hostile_time', 'first_cast_time') if key in unit]
        if boundaries and (min(boundaries) - timestamp_value(sample['time'])).total_seconds() < margin_seconds:
            continue
        output.write(json.dumps(sample, separators=(',', ':')) + '\n')


def minimum_assignment(costs):
    """Minimum-cost rectangular assignment, with one column per observation slot."""
    rows, columns = len(costs), len(costs[0])
    if rows > columns:
        raise ValueError('more observations than slots')
    u, v, matching, previous = [0.0] * (rows + 1), [0.0] * (columns + 1), [0] * (columns + 1), [0] * (columns + 1)
    for row in range(1, rows + 1):
        matching[0] = row
        column = 0
        best = [math.inf] * (columns + 1)
        used = [False] * (columns + 1)
        while True:
            used[column] = True
            current = matching[column]
            delta, next_column = math.inf, 0
            for candidate in range(1, columns + 1):
                if not used[candidate]:
                    value = costs[current - 1][candidate - 1] - u[current] - v[candidate]
                    if value < best[candidate]:
                        best[candidate], previous[candidate] = value, column
                    if best[candidate] < delta:
                        delta, next_column = best[candidate], candidate
            if not math.isfinite(delta):
                return None, math.inf
            for candidate in range(columns + 1):
                if used[candidate]:
                    u[matching[candidate]] += delta
                    v[candidate] -= delta
                else:
                    best[candidate] -= delta
            column = next_column
            if matching[column] == 0:
                break
        while column:
            prior = previous[column]
            matching[column] = matching[prior]
            column = prior
    assignment = [None] * rows
    for column in range(1, columns + 1):
        if matching[column]:
            assignment[matching[column] - 1] = column - 1
    return assignment, sum(costs[i][slot] for i, slot in enumerate(assignment))


def connected_observations(observations, radius):
    parents = list(range(len(observations)))
    cells = defaultdict(list)
    def root(index):
        while parents[index] != index:
            parents[index] = parents[parents[index]]
            index = parents[index]
        return index
    for index, point in enumerate(observations):
        cell = (math.floor(point['x'] / radius), math.floor(point['y'] / radius))
        for x in range(cell[0] - 1, cell[0] + 2):
            for y in range(cell[1] - 1, cell[1] + 2):
                for other in cells[(x, y)]:
                    if math.hypot(point['x'] - observations[other]['x'], point['y'] - observations[other]['y']) <= radius:
                        parents[root(index)] = root(other)
        cells[cell].append(index)
    groups = defaultdict(list)
    for index, point in enumerate(observations):
        groups[root(index)].append(point)
    return list(groups.values())


def assign_component(key, observations, margin_threshold):
    by_run = defaultdict(list)
    for point in observations:
        by_run[(point['file'], point['run_start'])].append(point)
    anchor_run = max(sorted(by_run), key=lambda run: len(by_run[run]))
    anchors = sorted(by_run[anchor_run], key=lambda point: point['guid'])
    slots = [{'map': key[0], 'npc_id': key[1], 'ui_map_id': key[2], 'ambiguous_match': False,
              'assignment_margin_yards': None, 'observations': [point],
              'candidate_observations': [point]} for point in anchors]
    for run in sorted(by_run):
        if run == anchor_run:
            continue
        points = sorted(by_run[run], key=lambda point: point['guid'])
        costs = [[math.hypot(point['x'] - anchor['x'], point['y'] - anchor['y']) for anchor in anchors] for point in points]
        assignment, best = minimum_assignment(costs)
        for row, slot in enumerate(assignment):
            slots[slot]['observations'].append(points[row])
            if points[row] not in slots[slot]['candidate_observations']:
                slots[slot]['candidate_observations'].append(points[row])
            if len(anchors) > 1:
                alternative = [list(values) for values in costs]
                alternative[row][slot] = math.inf
                alternative_assignment, next_best = minimum_assignment(alternative)
                margin = next_best - best
                if math.isfinite(margin):
                    affected = {assigned for i, assigned in enumerate(assignment)
                                if assignment[i] != alternative_assignment[i]}
                    affected.update(assigned for i, assigned in enumerate(alternative_assignment)
                                    if assignment[i] != alternative_assignment[i])
                    for affected_slot in affected:
                        candidate = slots[affected_slot]
                        prior = candidate['assignment_margin_yards']
                        candidate['assignment_margin_yards'] = margin if prior is None else min(prior, margin)
                        if margin < margin_threshold:
                            candidate['ambiguous_match'] = True
                    if margin < margin_threshold:
                        for i, alternate_slot in enumerate(alternative_assignment):
                            if assignment[i] != alternate_slot:
                                candidate = slots[alternate_slot]['candidate_observations']
                                if points[i] not in candidate:
                                    candidate.append(points[i])
    return slots


def cluster_population(runs, radius=10.0, margin_threshold=1.0):
    """Assign observations to maximum-per-run slots within connected 2-D areas."""
    groups = defaultdict(list)
    for run in runs:
        for guid, unit in sorted(run['creatures'].items()):
            pos = unit.get('first_position')
            if pos is None or unit['summoned'] or unit.get('player_controlled', False) or unit.get('control_flags_invalid', False):
                continue
            key = (run['map'], unit['npc_id'], pos['ui_map_id'])
            groups[key].append({'file': run['file'], 'run_start': run['start'], 'guid': guid,
                'spawn_uid': unit['spawn_uid'], 'x': pos['x'], 'y': pos['y'], 'time': pos['time'], 'event': pos['event'],
                'facing': pos['facing'], 'level': pos['level'], 'summoned': unit['summoned']})
    clusters = []
    for key, observations in sorted(groups.items()):
        for component in connected_observations(observations, radius):
            clusters.extend(assign_component(key, component, margin_threshold))
    for cluster in clusters:
        observations = cluster['observations']
        spread = max((math.hypot(a['x'] - b['x'], a['y'] - b['y'])
                      for a in observations for b in observations), default=0.0)
        cluster['spread_xy'] = spread
        if cluster['ambiguous_match']:
            candidates = cluster['candidate_observations']
            candidate_spread = max((math.hypot(a['x'] - b['x'], a['y'] - b['y'])
                                    for a in candidates for b in candidates), default=0.0)
            cluster['candidate_spread_yards'] = candidate_spread
            if candidate_spread <= 5.0:
                cluster['candidate_centroid'] = {'x': sum(p['x'] for p in candidates) / len(candidates),
                                                  'y': sum(p['y'] for p in candidates) / len(candidates)}
        if len(observations) < 2 or cluster['ambiguous_match']:
            cluster['classification'] = 'insufficient_or_ambiguous'
        elif spread > 5:
            cluster['classification'] = 'patrol_candidate'
        else:
            cluster['classification'] = 'stationary_candidate'
        if cluster['classification'] == 'patrol_candidate':
            remaining = list(observations)
            route = [remaining.pop(0)]
            while remaining:
                last = route[-1]
                nearest = min(range(len(remaining)), key=lambda i: (remaining[i]['x'] - last['x']) ** 2 + (remaining[i]['y'] - last['y']) ** 2)
                route.append(remaining.pop(nearest))
            cluster['unverified_candidate_path'] = [{'x': point['x'], 'y': point['y']} for point in route]
            cluster['gap'] = 'PARTIAL_PATROL_NAVMESH_AND_PHASE_UNVERIFIED'
    return clusters


def centroid_requests(clusters):
    requests = []
    for index, cluster in enumerate(clusters):
        centroid = cluster.get('candidate_centroid')
        if centroid is None:
            continue
        first = cluster['observations'][0]
        requests.append({'cluster_index': index, 'file': first['file'], 'run_start': first['run_start'],
                         'guid': first['guid'], 'map': cluster['map'], 'x': centroid['x'], 'y': centroid['y'],
                         'ui_map_id': cluster['ui_map_id'], 'neighbor_hints': cluster['candidate_observations']})
    return requests


def cluster_safe_samples(runs, samples):
    units = {(run['file'], run['start'], guid): unit
             for run in runs for guid, unit in run['creatures'].items()}
    selected = {}
    for line in samples:
        sample = json.loads(line)
        key = (sample['file'], sample['run_start'], sample['guid'])
        if key not in units:
            raise ValueError(f'sample has no census unit: {key}')
        if key not in selected or sample['seconds_from_start'] < selected[key]['seconds_from_start']:
            selected[key] = sample
    safe_runs = []
    for run in runs:
        copy = {'file': run['file'], 'start': run['start'], 'map': run['map'], 'creatures': {}}
        for guid, unit in run['creatures'].items():
            key = (run['file'], run['start'], guid)
            if key in selected:
                copy['creatures'][guid] = {**unit, 'first_position': selected[key]}
        safe_runs.append(copy)
    return cluster_population(safe_runs)


def summarize_population(runs, database=None):
    maps = {}
    for run in runs:
        group = maps.setdefault(run['map'], {'map': run['map'], 'runs': 0, 'npcs': {}})
        group['runs'] += 1
        counts = defaultdict(int)
        for unit in run['creatures'].values():
            entry = group['npcs'].setdefault(unit['npc_id'], {'npc_id': unit['npc_id'],
                'names': [], 'observed_per_run': [], 'summon_destination_count': 0,
                'first_seen_in_encounter_count': 0, 'observed_outside_encounter_count': 0})
            if unit['name'] not in entry['names']:
                entry['names'].append(unit['name'])
            counts[unit['npc_id']] += 1
            entry['summon_destination_count'] += int(unit['summoned'])
            entry['first_seen_in_encounter_count'] += int(unit['first_encounter_id'] is not None)
            entry['observed_outside_encounter_count'] += int(unit['seen_outside_encounter'])
        for npc_id, count in counts.items():
            group['npcs'][npc_id]['observed_per_run'].append({'file': run['file'], 'start': run['start'], 'count': count})
    if database:
        with sqlite3.connect(database.resolve().as_uri() + '?mode=ro', uri=True) as conn:
            for map_id, group in maps.items():
                group['authored'] = {}
                for table, column in (('tdb_creature', 'id'), ('content_creature', 'id1')):
                    rows = conn.execute(f'SELECT {column}, count(*) FROM {table} WHERE map=? GROUP BY {column}', (map_id,)).fetchall()
                    group['authored'][table] = {'count': sum(count for _, count in rows),
                                                'npc_counts': dict(rows)}
    for group in maps.values():
        group['npcs'] = sorted(group['npcs'].values(), key=lambda item: item['npc_id'])
    return [maps[key] for key in sorted(maps)]


def summarize_encounters(runs):
    grouped = defaultdict(list)
    for run in runs:
        for encounter in run['encounters']:
            if encounter.get('success'):
                grouped[(run['map'], encounter['id'], encounter['name'])].append(encounter)
    result = []
    for (map_id, encounter_id, name), kills in sorted(grouped.items()):
        spells = defaultdict(lambda: {'first_cast_seconds': [], 'interval_seconds': []})
        for kill in kills:
            by_actor = defaultdict(list)
            for cast in kill['casts']:
                if cast.get('known_boss'):
                    unit = kill['units'][cast['guid']]
                    key = (unit['npc_id'], cast['spell_id'], cast['event'])
                    by_actor[(key, cast['guid'])].append(cast['seconds_from_pull'])
            for (key, _), times in by_actor.items():
                times.sort()
                spells[key]['first_cast_seconds'].append(times[0])
                spells[key]['interval_seconds'].extend(b - a for a, b in zip(times, times[1:]))
        result.append({'map': map_id, 'encounter_id': encounter_id, 'name': name,
                       'successful_pulls': len(kills),
                       'spells': [{'npc_id': key[0], 'spell_id': key[1], 'event': key[2], **values,
                                   'interval_range_seconds': [min(values['interval_seconds']), max(values['interval_seconds'])]
                                   if values['interval_seconds'] else None}
                                  for key, values in sorted(spells.items())]})
    return result


def read_trash_timers(path):
    """Read the generated addon data as text, without executing its Lua."""
    timers = {}
    challenge = npc = spell = None
    for number, line in enumerate(path.read_text(encoding='utf-8').splitlines(), 1):
        match = re.match(r'^( *)(?:\[(\d+)\] = \{|(first|cd) = (.*),)$', line)
        if not match:
            continue
        indent, key, field, value = match.groups()
        if key is not None:
            if len(indent) == 4:
                challenge = int(key)
            elif len(indent) == 12:
                npc = int(key)
            elif len(indent) == 20:
                spell = {'spell_id': int(key), 'source': str(path), 'line': number}
                timers.setdefault((challenge, npc), []).append(spell)
        elif len(indent) == 24 and spell is not None:
            if field == 'first' and re.fullmatch(r'-?\d+(?:\.\d+)?', value):
                spell['first_seconds'] = float(value)
            elif field == 'cd' and re.fullmatch(r'\{[\d., ]*\}', value):
                spell['interval_seconds'] = [float(x.strip()) for x in value[1:-1].split(',') if x.strip()]
    return timers


def annotate_trash(runs, timers):
    for run in runs:
        for creature in run['creatures'].values():
            spells = timers.get((run['challenge_id'], creature['npc_id']))
            if spells:
                creature['addon_trash_timers'] = spells


def floor_requests(runs):
    requests = []
    for run in runs:
        for guid, creature in run['creatures'].items():
            position = creature.get('first_position')
            if position is None:
                continue
            requests.append({
                'file': run['file'], 'run_start': run['start'], 'guid': guid,
                'map': run['map'], 'x': position['x'], 'y': position['y'],
                'ui_map_id': position['ui_map_id'],
                'neighbor_hints': creature['position_neighbors'],
            })
    return requests


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--map", type=int, action="append", required=True)
    parser.add_argument("--zone-only", action='store_true', help='extract zone-entry intervals outside challenge keys')
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--floor-requests", type=Path)
    parser.add_argument("--precombat-samples", type=Path)
    parser.add_argument("--safe-samples", type=Path)
    parser.add_argument("--safe-clusters", type=Path)
    parser.add_argument("--trash-data", type=Path)
    parser.add_argument("--world-db", type=Path)
    parser.add_argument("--encounter-summary", type=Path)
    parser.add_argument("--population-summary", type=Path)
    parser.add_argument("--clusters", type=Path)
    parser.add_argument("--centroid-requests", type=Path)
    parser.add_argument("logs", type=Path, nargs="+")
    args = parser.parse_args()
    paths = [f for p in args.logs for f in (sorted(p.glob("*.txt")) if p.is_dir() else [p])]
    if args.precombat_samples:
        args.precombat_samples.parent.mkdir(parents=True, exist_ok=True)
        with args.precombat_samples.open('w', encoding='utf-8') as samples:
            runs = census(paths, set(args.map), samples, args.zone_only)
    else:
        runs = census(paths, set(args.map), zones_only=args.zone_only)
    if args.safe_samples:
        if not args.precombat_samples:
            parser.error('--safe-samples requires --precombat-samples')
        args.safe_samples.parent.mkdir(parents=True, exist_ok=True)
        with args.precombat_samples.open(encoding='utf-8') as source, args.safe_samples.open('w', encoding='utf-8') as output:
            filter_safe_samples(runs, source, output)
    if args.safe_clusters:
        if not args.safe_samples:
            parser.error('--safe-clusters requires --safe-samples')
        args.safe_clusters.parent.mkdir(parents=True, exist_ok=True)
        with args.safe_samples.open(encoding='utf-8') as source:
            safe_clusters = cluster_safe_samples(runs, source)
        args.safe_clusters.write_text(json.dumps(safe_clusters, indent=2) + '\n', encoding='utf-8')
    if args.trash_data:
        annotate_trash(runs, read_trash_timers(args.trash_data))
    if args.world_db:
        annotate_bosses(runs, args.world_db)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps({"runs": runs}, indent=2) + "\n", encoding="utf-8")
    if args.floor_requests:
        args.floor_requests.parent.mkdir(parents=True, exist_ok=True)
        args.floor_requests.write_text(json.dumps(floor_requests(runs), indent=2) + "\n", encoding="utf-8")
    if args.encounter_summary:
        args.encounter_summary.parent.mkdir(parents=True, exist_ok=True)
        args.encounter_summary.write_text(json.dumps(summarize_encounters(runs), indent=2) + "\n", encoding="utf-8")
    if args.population_summary:
        args.population_summary.parent.mkdir(parents=True, exist_ok=True)
        args.population_summary.write_text(json.dumps(summarize_population(runs, args.world_db), indent=2) + "\n", encoding="utf-8")
    if args.clusters or args.centroid_requests:
        clusters = cluster_population(runs)
        if args.clusters:
            args.clusters.parent.mkdir(parents=True, exist_ok=True)
            args.clusters.write_text(json.dumps(clusters, indent=2) + "\n", encoding="utf-8")
        if args.centroid_requests:
            args.centroid_requests.parent.mkdir(parents=True, exist_ok=True)
            args.centroid_requests.write_text(json.dumps(centroid_requests(clusters), indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
