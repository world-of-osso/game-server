#!/usr/bin/env python3
"""Derive source-buff-filtered WCL +2-reference melee modifiers from a report manifest."""
import argparse
import collections
import json
import math
import sqlite3
from pathlib import Path

# Applied aura types that can directly increase melee damage or attack power.
DAMAGE_AURAS = {13, 79, 99, 166, 168, 344}
CLASS_MODS = {1: 4, 2: 2, 4: 3, 8: 1}


def quantile(values, q):
    values = sorted(values)
    index = (len(values) - 1) * q
    low = int(index)
    high = min(low + 1, len(values) - 1)
    return values[low] + (values[high] - values[low]) * (index - low)


def coefficient(level):
    return math.floor((1.07 ** min(max(level - 1, 0), 9) * 1.10 ** max(level - 10, 0)) * 100 + 0.5) / 100


def source_buff_intervals(events, buff_ids, fight_start, fight_end):
    streams = collections.defaultdict(list)
    for e in events:
        if e.get('abilityGameID') in buff_ids:
            streams[(e['targetID'], e.get('targetInstance', 1), e['abilityGameID'])].append(e)
    intervals = collections.defaultdict(list)
    for (actor, instance, spell), stream in streams.items():
        active = None
        for e in sorted(stream, key=lambda e: e['timestamp']):
            kind, at = e['type'], e['timestamp']
            if kind == 'applybuff' and active is None:
                active = at
            elif kind in ('applybuffstack', 'refreshbuff', 'removebuffstack') and active is None:
                active = fight_start  # The start of this already-active aura is unobserved.
            elif kind == 'removebuff':
                intervals[(actor, instance)].append((active if active is not None else fight_start, at, spell))
                active = None
        if active is not None:
            intervals[(actor, instance)].append((active, fight_end, spell))
    return intervals


def seasonal_modifiers(db, tuning, season):
    milestone = db.execute('SELECT MilestoneSeason FROM mythic_plus_season WHERE ID=?', (season,)).fetchone()
    if not milestone:
        raise ValueError(f'missing season {season}')
    milestones = dict(db.execute('SELECT ID,MilestoneSeason FROM mythic_plus_season'))
    rows = db.execute('SELECT ExpectedStatModID,MinMythicPlusSeasonID,MaxMythicPlusSeasonID FROM content_tuning_x_expected WHERE ContentTuningID=? ORDER BY ID', (tuning,)).fetchall()
    if not rows:
        raise ValueError(f'missing tuning {tuning}')
    result = []
    for mod, minimum, maximum in rows:
        if (minimum and milestone[0] < milestones[minimum]) or (maximum and milestone[0] >= milestones[maximum]):
            continue
        factor = db.execute('SELECT CreatureAutoAttackDPSMod FROM expected_stat_mod WHERE ID=?', (mod,)).fetchone()
        if factor is None:
            raise ValueError(f'missing modifier {mod}')
        result.append((mod, factor[0]))
    return result


def baseline(db, map_id, entry, season):
    row = db.execute('''SELECT d.maxlevel,d.LevelScalingDeltaMin,d.LevelScalingDeltaMax,
       d.HealthScalingExpansion,d.DamageModifier,t.unit_class,t.name
       FROM content_creature_template_difficulty d JOIN content_creature_template t ON t.entry=d.entry
       WHERE d.entry=? AND d.DifficultyID=8''', (entry,)).fetchone()
    if not row:
        return None, 'no difficulty-8 content row'
    max_level, low, high, expansion, old, unit_class, name = row
    if low != high:
        return None, 'variable spawn level delta'
    tuning = db.execute('SELECT ContentTuningID FROM map_difficulty WHERE MapID=? AND DifficultyID=8', (map_id,)).fetchone()
    if not tuning:
        return None, 'missing difficulty-8 map tuning'
    cap = db.execute('SELECT maxlevel FROM content_tuning_level WHERE ID=?', (tuning[0],)).fetchone()
    if not cap:
        return None, 'missing map tuning level cap'
    level = cap[0] + low
    expansion = 11 if expansion == -1 else expansion
    stat = db.execute('SELECT CreatureAutoAttackDps FROM expected_stat WHERE Lvl=? AND ExpansionID IN (?, -2) ORDER BY (ExpansionID=?) DESC,ID DESC LIMIT 1', (level, expansion, expansion)).fetchone()
    if not stat:
        return None, 'missing ExpectedStat'
    mods = seasonal_modifiers(db, tuning[0], season)
    class_mod = CLASS_MODS.get(unit_class)
    if class_mod:
        factor = db.execute('SELECT CreatureAutoAttackDPSMod FROM expected_stat_mod WHERE ID=?', (class_mod,)).fetchone()
        if not factor:
            return None, 'missing class modifier'
        mods.append((class_mod, factor[0]))
    # LevelStats applies the template DamageModifier later; this denominator omits it.
    value = stat[0] * math.prod(f for _, f in mods) * 2.0 * 1.25
    return {'level': level, 'name': name, 'prior_value': old, 'base_median': value,
            'expected_stat_dps': stat[0], 'map_tuning': tuning[0], 'modifiers': mods}, None


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--manifest', type=Path, required=True)
    ap.add_argument('--database', type=Path, required=True)
    ap.add_argument('--output', type=Path, required=True)
    args = ap.parse_args()
    root = args.manifest.resolve().parent
    manifest = json.loads(args.manifest.read_text())
    db = sqlite3.connect(f'file:{args.database.resolve()}?mode=ro', uri=True)
    samples = collections.defaultdict(list)
    excluded = collections.Counter()
    aura_names = {}
    omissions = collections.Counter()
    for report in manifest['reports']:
        label = report['label']
        meta = json.loads((root / report['metadata']).read_text())['reportData']['report']
        fight = next(f for f in meta['fights'] if f['id'] == report['fight'])
        actors = {a['id']: a for a in meta['masterData']['actors']}
        buffs = [json.loads(line) for line in (root / f'{label}-buffs.jsonl').open() if line.strip()]
        spells = {e['abilityGameID'] for e in buffs if actors.get(e.get('targetID'), {}).get('type') == 'NPC'}
        risky = set()
        for spell in spells:
            effects = db.execute('SELECT EffectAura,EffectBasePointsF FROM spell_effect WHERE SpellID=?', (spell,)).fetchall()
            name = db.execute('SELECT Name_lang FROM spell_name WHERE ID=?', (spell,)).fetchone()
            if not effects or any(aura in DAMAGE_AURAS and points > 0 for aura, points in effects):
                risky.add(spell)
                aura_names[spell] = name[0] if name else 'unknown aura (conservatively excluded)'
        intervals = source_buff_intervals(buffs, risky, fight['startTime'], fight['endTime'])
        for line in (root / f'{label}-damage.jsonl').open():
            e = json.loads(line)
            if e.get('type') != 'damage' or e.get('abilityGameID') != 1:
                continue
            source = actors.get(e.get('sourceID'), {})
            target = actors.get(e.get('targetID'), {})
            if source.get('type') != 'NPC' or not source.get('gameID') or target.get('type') not in ('Player', 'Pet'):
                continue
            value = e.get('unmitigatedAmount')
            if value is None or value <= 0:
                omissions[(label, 'no positive unmitigatedAmount')] += 1
                continue
            active = [spell for start, end, spell in intervals.get((e['sourceID'], e.get('sourceInstance', 1)), []) if start <= e['timestamp'] <= end]
            if active:
                for spell in active:
                    excluded[(report['map'], source['gameID'], spell)] += 1
                continue
            normalized = value / (coefficient(report['level']) / coefficient(2))
            samples[(report['map'], source['gameID'])].append((normalized, label, e.get('hitType')))
    by_entry = collections.defaultdict(list)
    result = []
    skipped = []
    for (map_id, entry), hits in sorted(samples.items()):
        labels = sorted({label for _, label, _ in hits})
        if len(labels) < 3:
            skipped.append({'map': map_id, 'entry': entry, 'reason': 'fewer than 3 reports', 'reports': labels, 'hits': len(hits)})
            continue
        base, reason = baseline(db, map_id, entry, manifest['season_id'])
        if reason:
            skipped.append({'map': map_id, 'entry': entry, 'reason': reason, 'reports': labels, 'hits': len(hits)})
            continue
        normalized = [value for value, _, _ in hits]
        modifier = quantile(normalized, 0.5) / base['base_median']
        row = {'map': map_id, 'entry': entry, **base, 'value': round(modifier, 6), 'n': len(hits),
               'reports': [{'label': label, 'code': next(r['code'] for r in manifest['reports'] if r['label'] == label), 'fight': next(r['fight'] for r in manifest['reports'] if r['label'] == label), 'level': next(r['level'] for r in manifest['reports'] if r['label'] == label), 'n': sum(1 for _, source, _ in hits if source == label)} for label in labels],
               'iqr': [quantile(normalized, .25)/base['base_median'], quantile(normalized, .75)/base['base_median']],
               'excluded_auras': [{'id': spell, 'name': aura_names[spell], 'excluded_hits': count} for (m, e, spell), count in sorted(excluded.items()) if m == map_id and e == entry],
               'provenance': 'INFERRED_WCL', 'retire_when': 'replace with sourced server DamageModifier when available'}
        result.append(row); by_entry[entry].append(map_id)
    duplicates = {entry: maps for entry, maps in by_entry.items() if len(maps) > 1}
    if duplicates:
        raise ValueError(f'entry appears on multiple maps, reconcile before SQL: {duplicates}')
    output = {'season_id': manifest['season_id'], 'method': 'All positive WCL melee unmitigatedAmount hits against players/pets, normalized to +2 by rounded Q(level)/Q(2); exclude known positive damage/attack-power and unknown source aura intervals. IQR is of normalized per-hit modifier values.', 'overrides': result, 'skipped': skipped, 'omissions': [{'report': k[0], 'reason': k[1], 'count': v} for k, v in sorted(omissions.items())], 'damage_aura_ids': aura_names}
    args.output.write_text(json.dumps(output, indent=2) + '\n')
    print(f'overrides={len(result)} skipped={len(skipped)} exclusions={sum(excluded.values())} unknown_raw={sum(omissions.values())}')
    for row in result:
        print(row['map'], row['entry'], row['name'], 'modifier', row['value'], 'n', row['n'], 'IQR', [round(x, 4) for x in row['iqr']], 'excluded', row['excluded_auras'])


if __name__ == '__main__':
    main()
