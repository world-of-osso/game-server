#!/usr/bin/env python3
"""Collect WCL creature cast timings without treating multi-target events as recasts.

Refuses two kinds of spell as creature timers:
- a spell whose aura lands on a player or pet and never expires. The server treats
  DurationIndex <= 0 or a missing duration as no expiry, so a timer stacks it forever.
- a spell with the area-entry signature: one source instance records it on two or more
  players/pets within ENTRY_WINDOW_MS and has no begincast for it. A creature cast
  logs one record per cast; such clusters are ground-area entry applications (Den
  Rotten Ground 1297701: 23 sub-500 ms pairs, 13 of them at most 20 ms).
"""
import argparse
import collections
import json
import sqlite3
from pathlib import Path

AURA_EFFECTS = {6, 27, 35, 65, 119, 128, 129, 143, 174, 202}
CASTER_ONLY_TARGETS = (1, 0)
ENTRY_WINDOW_MS = 20
ENEMY_ACTOR_TYPES = {'Player', 'Pet'}


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


def read_nonexpiring_auras(db, spell):
    """Aura effects that can land on a non-caster when the base duration does not expire."""
    duration = db.execute(
        'SELECT d.Duration FROM spell_misc m JOIN spell_duration d ON d.ID=m.DurationIndex '
        'WHERE m.SpellID=? AND m.DifficultyID=0',
        (spell,),
    ).fetchone()
    if duration and duration[0] > 0:
        return []
    rows = db.execute(
        'SELECT DifficultyID,EffectIndex,Effect,EffectAura,ImplicitTarget_0,ImplicitTarget_1 FROM spell_effect '
        'WHERE SpellID=? AND EffectAura<>0 ORDER BY DifficultyID,EffectIndex',
        (spell,),
    ).fetchall()
    return [
        {'difficulty': difficulty, 'effect_index': index, 'aura': aura}
        for difficulty, index, effect, aura, first, second in rows
        if effect in AURA_EFFECTS and (first, second) != CASTER_ONLY_TARGETS
    ]


def area_entry_clusters(events):
    """Records on distinct players/pets within ENTRY_WINDOW_MS of each other."""
    clusters = []
    for earlier, later in zip(events, events[1:]):
        if (
            later[0] - earlier[0] <= ENTRY_WINDOW_MS
            and earlier[1] != later[1]
            and earlier[2] in ENEMY_ACTOR_TYPES
            and later[2] in ENEMY_ACTOR_TYPES
        ):
            clusters.append({'fight_ms': earlier[0], 'separation_ms': later[0] - earlier[0]})
    return clusters


def refusal_reasons(db, result):
    reasons = []
    if result['enemy_target_casts']:
        auras = read_nonexpiring_auras(db, result['spell'])
        if auras:
            reasons.append({'reason': 'non_expiring_aura_on_enemy', 'effects': auras})
    if result['area_entry_clusters']:
        reasons.append({'reason': 'area_entry_signature', 'clusters': result['area_entry_clusters']})
    return reasons


def split_refused(db, results):
    """Timer rows the emitter may use, and refused spells with their evidence."""
    spells, refused = [], []
    for result in results.values():
        reasons = refusal_reasons(db, result)
        if reasons:
            refused.append({'entry': result['entry'], 'spell': result['spell'], 'reasons': reasons})
        else:
            spells.append(result)
    return spells, refused


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
    begincasts = set()
    for event in read_events(Path(f'{prefix}-casts.jsonl')):
        actor = actors.get(event.get('sourceID'), {})
        if event.get('type') not in ('cast', 'begincast') or actor.get('type') != 'NPC' or actor.get('subType') == 'Boss' or not actor.get('gameID'):
            continue
        actor_key = (event['sourceID'], event.get('sourceInstance', 1))
        key = (actor['gameID'], event['abilityGameID'], actor_key)
        if event['type'] == 'begincast':
            begincasts.add(key)
            continue
        target_type = actors.get(event.get('targetID'), {}).get('type')
        casts[key].append((event['fight_ms'], event.get('targetID'), target_type))

    for (entry, spell, actor), events in casts.items():
        events.sort(key=lambda event: event[0])
        result = results.setdefault((entry, spell), {
            'entry': entry, 'spell': spell, 'source': 'INFERRED_WCL',
            'db2_bound': read_spell_bound(db, spell),
            'delay_proxy_ms': [], 'interval_ms': [],
            'enemy_target_casts': 0, 'area_entry_clusters': [],
        })
        source = {'report': report, 'actor': actor}
        result['enemy_target_casts'] += sum(event[2] in ENEMY_ACTOR_TYPES for event in events)
        if (entry, spell, actor) not in begincasts:
            result['area_entry_clusters'].extend({**source, **cluster} for cluster in area_entry_clusters(events))
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
    spells, refused = split_refused(db, results)
    data = {
        'warning': 'First damage activity is a proxy, not necessarily combat entry. DB2 bound uses available base-difficulty recovery and cast-time fields; missing fields are unknown. Successive casts can be interrupted or delayed.',
        'retirement_condition': 'Replace INFERRED_WCL behavior when authoritative creature behavior is verified.',
        'spells': spells,
        'refused': refused,
    }
    args.output.write_text(json.dumps(data, indent=2) + '\n')


if __name__ == '__main__':
    main()
