#!/usr/bin/env python3
"""Convert the Altar WCL pull's damage-taken resources to replay pressure.

Input: `wcl events pPFDbkfQK7MACtmY --fight 41 --pull 1 --type
DamageTaken --hostility Friendlies --resources`. The WCL max HP observations
must agree per player; non-damaging outcomes stay in the trace for audit.
"""
import argparse
import collections
import json
import sqlite3
from pathlib import Path

ACTORS = {6: 0, 221: 1, 1: 2, 225: 3, 231: 4}
DEFERRED_ENEMY_DAMAGE = {124255, 361029}


def read_max_health(events):
    observations = collections.defaultdict(set)
    for event in events:
        if event.get('maxHitPoints', 0) > 0:
            observations[event['targetID']].add(event['maxHitPoints'])
    if any(len(observations[actor]) != 1 for actor in ACTORS):
        raise ValueError(f'expected one observed max HP per actor: {dict(observations)}')
    return {actor: next(iter(observations[actor])) for actor in ACTORS}


def read_spell_metadata(db, spell):
    if spell == 1:
        return 1, 2  # WCL basic melee sentinel, not SpellInfo ID 1.
    row = db.execute(
        'SELECT SchoolMask FROM spell_misc WHERE SpellID=? AND DifficultyID=0', (spell,)
    ).fetchone()
    if row is None:
        raise ValueError(f'missing spell school {spell}')
    school = row[0]
    row = db.execute(
        'SELECT DefenseType FROM spell_categories WHERE SpellID=? AND DifficultyID=0', (spell,)
    ).fetchone()
    return school, row[0] if row else None


def convert(events, db):
    max_health = read_max_health(events)
    hits = []
    observed_self = []
    excluded_self = 0
    for event in events:
        target = event['targetID']
        if target not in ACTORS:
            raise ValueError(f'unknown WCL party target {target}')
        source = event['sourceID']
        spell = event['abilityGameID']
        deferred = source == target and spell in DEFERRED_ENEMY_DAMAGE
        if source == target:
            observed_self.append({'time_ms': int(event['fight_ms']), 'slot': ACTORS[target],
                                  'spell_id': spell, 'amount': event.get('amount', 0),
                                  'absorbed': event.get('absorbed', 0), 'max_hp': max_health[target]})
            if not deferred:
                excluded_self += 1
                continue
        elif source in ACTORS:
            raise ValueError(f'unmapped friendly damage source {source}')
        school, defense = read_spell_metadata(db, spell)
        hits.append({'time_ms': int(event['fight_ms']), 'slot': ACTORS[target],
                     'source_actor': source, 'target_actor': target, 'spell_id': spell,
                     'school': school, 'defense': defense, 'hit_type': event['hitType'],
                     'periodic': event.get('tick', False), 'deferred': deferred, 'amount': event.get('amount', 0),
                     'absorbed': event.get('absorbed', 0), 'max_hp': max_health[target],
                     'health_after': event.get('hitPoints')})
    if not hits or any(a['time_ms'] > b['time_ms'] for a, b in zip(hits, hits[1:])):
        raise ValueError('WCL damage events must be nonempty and ordered')
    return {'provenance': 'INFERRED_WCL pPFDbkfQK7MACtmY fight 41 pull 1',
            'actor_slots': ACTORS, 'party_specs': [268, 1468, 62, 262, 262],
            'excluded_self': excluded_self, 'observed_self': observed_self, 'hits': hits}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('events', type=Path)
    parser.add_argument('world_db', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    events = [json.loads(line) for line in args.events.read_text().splitlines() if line.strip()]
    with sqlite3.connect(f'file:{args.world_db}?mode=ro', uri=True) as db:
        result = convert(events, db)
    args.output.write_text(json.dumps(result, separators=(',', ':')) + '\n')


if __name__ == '__main__':
    main()
