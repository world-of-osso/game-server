#!/usr/bin/env python3
"""Convert Altar WCL fight 41 pull 1 damage events to a healer replay trace.

The input is `wcl events pPFDbkfQK7MACtmY --fight 41 --pull 1 --type
DamageTaken --hostility Friendlies`. School and defense are from the selected
world snapshot, not inferred from the spell name. WCL reports ability 1 for a
swing. Avoided hits retain their hitType and have no invented raw amount.
"""
import argparse
import json
import sqlite3
from pathlib import Path

ACTORS = {6: 0, 221: 1, 1: 2, 225: 3, 231: 4}


def convert(events, db):
    hits = []
    for event in events:
        target = event['targetID']
        if target not in ACTORS:
            raise ValueError(f'unknown WCL party target {target}')
        source = event['sourceID']
        if source in ACTORS and source != target:
            raise ValueError(f'unmapped friendly damage source {source}')
        spell = event['abilityGameID']
        if spell == 1:
            school, defense = 1, 2
        else:
            row = db.execute('SELECT SchoolMask FROM spell_misc WHERE SpellID=? AND DifficultyID=0', (spell,)).fetchone()
            if row is None:
                raise ValueError(f'missing spell school {spell}')
            school = row[0]
            row = db.execute('SELECT DefenseType FROM spell_categories WHERE SpellID=? AND DifficultyID=0', (spell,)).fetchone()
            defense = row[0] if row else None
        raw = event.get('unmitigatedAmount')
        if raw is None and (event.get('amount', 0) or event.get('absorbed', 0)):
            raise ValueError(f'positive WCL damage without unmitigated amount: {event}')
        hits.append({'time_ms': int(event['fight_ms']), 'slot': ACTORS[target],
                     'source_actor': source, 'target_actor': target, 'spell_id': spell,
                     'school': school, 'defense': defense, 'hit_type': event['hitType'],
                     'periodic': event.get('tick', False), 'unmitigated': raw,
                     'amount': event.get('amount', 0), 'absorbed': event.get('absorbed', 0),
                     'mitigated': event.get('mitigated', 0)})
    if not hits or any(a['time_ms'] > b['time_ms'] for a, b in zip(hits, hits[1:])):
        raise ValueError('WCL damage events must be nonempty and ordered')
    return {'provenance': 'INFERRED_WCL pPFDbkfQK7MACtmY fight 41 pull 1',
            'actor_slots': ACTORS, 'hits': hits}


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
