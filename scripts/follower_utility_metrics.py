#!/usr/bin/env python3
"""Measure real simulator combat traces using FOLLOWER_UTILITY_POLICY danger classes."""
import argparse
from collections import Counter
from functools import cache
import json
from pathlib import Path
import sqlite3


class NativeDanger:
    def __init__(self, database):
        self.db = sqlite3.connect(f'file:{database}?mode=ro', uri=True)

    @cache
    def __call__(self, spell):
        pending, seen, danger = [spell], set(), 0
        while pending:
            spell = pending.pop()
            if spell in seen:
                continue
            seen.add(spell)
            rows = self.db.execute('SELECT Effect,EffectAura,EffectTriggerSpell FROM spell_effect WHERE SpellID=? AND DifficultyID=0', (spell,))
            for effect, aura, child in rows:
                if aura in (5, 7, 12, 26, 27, 60):
                    danger = max(danger, 4)
                elif effect in (10, 18, 67, 136, 172):
                    danger = max(danger, 3)
                elif effect in (2, 9, 17, 31, 58, 121) or aura in (3, 53, 89):
                    danger = max(danger, 2)
                else:
                    danger = max(danger, 1)
                if child:
                    pending.append(child)
        return danger


def measure(report, danger):
    events = report['performance']['combat_events']
    party = {member['entity'] for sample in report['samples'] for member in sample['members']}
    friendly = set(party)
    hostile = {unit['entity'] for unit in report['terminal_units'] if unit['guid'] is not None}
    for event in events:
        if event['kind'] == 'Summon':
            if event['source'] in friendly:
                friendly.add(event['target'])
            if event['source'] in hostile:
                hostile.add(event['target'])
    pending = {}
    completed = Counter()
    interrupts = Counter()
    deaths = 0
    for event in events:
        source, target, spell = event['source'], event['target'], event['spell_id']
        kind = event['kind']
        if kind == 'CastStart' and source in hostile:
            pending[source] = spell
        elif kind == 'CastSuccess' and source in hostile:
            started = pending.pop(source, None)
            if started == spell and danger(spell) >= 2:
                completed[spell] += 1
        elif kind == 'Interrupt':
            pending.pop(target, None)
            if source in friendly and target in hostile:
                interrupts[event['extra_spell_id']] += 1
        elif kind == 'Death':
            pending.pop(target, None)
            deaths += target in party
    return dict(seed=report['seed'], status=report['status'], interrupts_landed=sum(interrupts.values()),
                dangerous_casts_through=sum(completed.values()), deaths=deaths,
                interrupted_spells=dict(interrupts), completed_dangerous_spells=dict(completed))


def summarize(directory, danger):
    paths = sorted(directory.glob('run-*.json'))
    if not paths:
        raise ValueError(f'no completed simulator reports in {directory}')
    rows = [measure(json.loads(path.read_text()), danger) for path in paths]
    return dict(runs=rows, seeds=sorted(row['seed'] for row in rows),
                interrupts_landed=sum(row['interrupts_landed'] for row in rows),
                dangerous_casts_through=sum(row['dangerous_casts_through'] for row in rows),
                deaths=sum(row['deaths'] for row in rows),
                clears=sum(row['status'] == 'cleared' for row in rows))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--database', type=Path, required=True)
    parser.add_argument('--baseline', type=Path, required=True)
    parser.add_argument('--candidate', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    danger = NativeDanger(args.database.resolve(strict=True))
    before = summarize(args.baseline, danger)
    after = summarize(args.candidate, danger)
    if before['seeds'] != after['seeds']:
        raise ValueError('baseline/candidate seeds differ')
    report = dict(policy='FOLLOWER_UTILITY_POLICY', baseline=before, candidate=after,
                  definition='Actual Interrupt logs from party/owned summons to native hostiles; dangerous CastSuccess after CastStart at source danger>=2; actual Death logs targeting the five party actors. Instant casts are not interruptible-cast completions. No disappearance-as-death inference.')
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({key: {field: value[field] for field in ('interrupts_landed', 'dangerous_casts_through', 'deaths', 'clears')} for key, value in [('baseline', before), ('candidate', after)]}, indent=2))


if __name__ == '__main__':
    main()
