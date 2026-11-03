#!/usr/bin/env python3
"""Collect WCL creature cast timings without treating multi-target events as recasts."""
import argparse
import collections
import json
import sqlite3
from pathlib import Path


def successive_cast_intervals(events, minimum_ms):
    """Collapse multi-target cast records using the sourced DB2 minimum interval."""
    timestamps = sorted({event[0] for event in events})
    accepted = []
    for timestamp in timestamps:
        if not accepted or timestamp - accepted[-1] >= minimum_ms:
            accepted.append(timestamp)
    return [later - earlier for earlier, later in zip(accepted, accepted[1:])]


def read_spell_bound(db, spell):
    """Read the engine's base-difficulty SpellCooldowns, SpellMisc and SpellCastTimes."""
    cooldown = db.execute(
        'SELECT RecoveryTime,CategoryRecoveryTime,StartRecoveryTime FROM spell_cooldowns WHERE SpellID=? AND DifficultyID=0',
        (spell,),
    ).fetchone()
    misc = db.execute(
        'SELECT CastingTimeIndex FROM spell_misc WHERE SpellID=? AND DifficultyID=0',
        (spell,),
    ).fetchone()
    cast = None
    if misc:
        cast = db.execute('SELECT Base FROM spell_cast_times WHERE ID=?', (misc[0],)).fetchone()
    values = list(cooldown) if cooldown else []
    cast_ms = cast[0] if cast else None
    if any(value is None or value < 0 for value in values) or (cast_ms is not None and cast_ms < 0):
        raise ValueError(f'spell {spell} has invalid DB2 recovery or cast time')
    return {
        'minimum_interval_ms': (max(values) if values else 0) + (cast_ms if cast_ms is not None else 0),
        'recovery_time_ms': cooldown[0] if cooldown else None,
        'category_recovery_time_ms': cooldown[1] if cooldown else None,
        'start_recovery_time_ms': cooldown[2] if cooldown else None,
        'casting_time_index': misc[0] if misc else None,
        'cast_time_ms': cast_ms,
    }


def read_events(path):
    with path.open() as stream:
        for line in stream:
            if line.strip():
                yield json.loads(line)


def collect_report(prefix, report, results, db):
    actors = {
        actor['id']: actor
        for actor in json.loads(Path(f'{prefix}-actors.json').read_text())['reportData']['report']['masterData']['actors']
    }
    first_activity = {}
    for kind, actor_field, instance_field in [
        ('friendly-damage', 'targetID', 'targetInstance'),
        ('damage', 'sourceID', 'sourceInstance'),
    ]:
        for event in read_events(Path(f'{prefix}-{kind}.jsonl')):
            actor_id = event.get(actor_field)
            if actors.get(actor_id, {}).get('type') != 'NPC':
                continue
            key = (actor_id, event.get(instance_field, 1))
            first_activity[key] = min(first_activity.get(key, float('inf')), event['fight_ms'])

    casts = collections.defaultdict(list)
    for event in read_events(Path(f'{prefix}-casts.jsonl')):
        actor = actors.get(event.get('sourceID'), {})
        if event.get('type') != 'cast' or actor.get('type') != 'NPC' or actor.get('subType') == 'Boss' or not actor.get('gameID'):
            continue
        actor_key = (event['sourceID'], event.get('sourceInstance', 1))
        key = (actor['gameID'], event['abilityGameID'], actor_key)
        casts[key].append((event['fight_ms'], event.get('targetID')))

    for (entry, spell, actor), events in casts.items():
        events.sort(key=lambda event: event[0])
        result = results.setdefault((entry, spell), {
            'entry': entry, 'spell': spell, 'source': 'INFERRED_WCL',
            'db2_bound': read_spell_bound(db, spell),
            'delay_proxy_ms': [], 'interval_ms': [],
        })
        source = {'report': report, 'actor': actor}
        first = first_activity.get(actor)
        if first is not None and first <= events[0][0]:
            result['delay_proxy_ms'].append({**source, 'delay_ms': events[0][0] - first})
        result['interval_ms'].extend(
            {**source, 'interval_ms': interval}
            for interval in successive_cast_intervals(events, result['db2_bound']['minimum_interval_ms'])
        )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--reports', type=Path, required=True)
    parser.add_argument('--input-dir', type=Path, required=True)
    parser.add_argument('--map-name', required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--database', type=Path, required=True)
    args = parser.parse_args()
    results = {}
    db = sqlite3.connect(f'file:{args.database.resolve()}?mode=ro', uri=True)
    for row in json.loads(args.reports.read_text()):
        code, fight = row['report'], row['fight']
        collect_report(args.input_dir / f'{args.map_name}-{code}-{fight}', f'{code}/{fight}', results, db)
    data = {
        'warning': 'First damage activity is a proxy, not necessarily combat entry. DB2 bound uses available base-difficulty recovery and cast-time fields; missing fields are unknown. Successive casts can be interrupted or delayed.',
        'retirement_condition': 'Replace INFERRED_WCL behavior when authoritative creature behavior is verified.',
        'spells': list(results.values()),
    }
    args.output.write_text(json.dumps(data, indent=2) + '\n')


if __name__ == '__main__':
    main()
