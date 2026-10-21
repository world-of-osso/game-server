#!/usr/bin/env python3
"""Read-only utility census of the installed 70448 selected follower spellbooks."""
import argparse
import hashlib
import json
from pathlib import Path
import sqlite3

CATEGORIES = {
    'raid buff': {1126, 1459, 21562, 6673, 462854, 364342},
    'lust': {2825, 32182, 80353, 390386},
    'control / root / knockback (authored identity; dummy execution unqualified)': {118, 853, 122, 339, 22570, 33786, 102359, 102793, 132469, 187650, 187698, 1715, 46968, 8122, 5782, 6789, 30283, 115078, 119381, 116844, 116095, 358385, 368970, 51490, 192058, 51485, 2484, 195645, 157980, 157997, 120},
    'combat resurrection': {20484, 391054, 20707},
    'movement / gap closer': {100, 6544, 385952, 190784, 1044, 1850, 252216, 102401, 106898, 1953, 781, 186257, 58875, 192077, 116841, 109132, 101545, 101643, 358267, 370665, 73325, 121536, 111400, 111771},
    'threat drop': {66, 110959, 5384, 586},
    'taunt (tank only)': {355, 62124, 1161, 386071, 6795, 115546},
    'external defensive': {1022, 6940, 204018, 102342, 47788, 97462},
    'personal defensive': {642, 31850, 86659, 871, 23920, 22812, 61336, 22842, 19236, 108271, 45438, 235313, 342245, 414658, 186265, 264735, 363916, 122470, 104773, 108416},
    'pet utility': {136, 883, 982, 1515, 2641, 6991, 19577, 83242, 83243, 83244, 83245, 93322, 272651, 321297, 688, 691, 697, 30146, 366222, 93375, 119898, 333889, 1098},
}


def category(row, effects):
    for name, ids in CATEGORIES.items():
        if row['spell_id'] in ids:
            return name
    if row['role_slot'] == 'passive':
        return 'passive (not autonomous)'
    if row['role_slot'] == 'interrupt' or any(e['Effect'] == 68 for e in effects):
        return 'interrupt'
    if row['role_slot'] == 'dispel' or any(e['Effect'] == 38 for e in effects):
        return 'dispel / purge / soothe'
    if row['role_slot'] == 'control' or any(e['EffectAura'] in (5, 7, 12, 26) for e in effects):
        return 'control / root'
    if row['role_slot'] in ('attack', 'aoe', 'heal', 'hot', 'mitigation', 'buff'):
        return 'rotation / maintenance (existing policy)'
    return 'other learned active (manual / not scheduled)'


def census(database):
    with sqlite3.connect(f'file:{database}?mode=ro', uri=True) as db:
        db.row_factory = sqlite3.Row
        db.execute('BEGIN')
        profiles = [dict(r) for r in db.execute('SELECT * FROM follower_profiles ORDER BY spec_id')]
        rows = [dict(r) for r in db.execute('SELECT k.*, n.Name_lang AS name FROM follower_spec_kit k JOIN spell_name n ON n.ID=k.spell_id ORDER BY k.spec_id,k.spell_id,k.min_level')]
        for row in rows:
            row['effects'] = [dict(r) for r in db.execute('SELECT EffectIndex,Effect,EffectAura,EffectMiscValue_0,EffectTriggerSpell,ImplicitTarget_0,ImplicitTarget_1 FROM spell_effect WHERE SpellID=? AND DifficultyID=0 ORDER BY EffectIndex', (row['spell_id'],))]
            row['category'] = category(row, row['effects'])
    return dict(build='12.1.5.70448', profiles=profiles, spells=rows)


def render(data):
    receipt = hashlib.sha256(json.dumps(data, sort_keys=True).encode()).hexdigest()
    lines = ['# Follower utility census — 2026-10-21', '', 'Source: read-only server world.db; selected native 12.1.5.70448 starter/default loadout intervals already published in follower_spec_kit. This is the union over levels1–90, not simultaneous learned availability. Every row is retained below, including passive/rotation exclusions so unclassified utility-slot attacks cannot disappear from the census.', '', f'Canonical JSON SHA-256: `{receipt}`.', '', 'Spell identities/effects/level intervals are server data. Automation timing/recipients are **FOLLOWER_UTILITY_POLICY**, not Retail follower behavior. No spell is granted by this census.', '', 'Bloodlust2825/Heroism32182 are absent from the selected Elemental spellbook; do not invent them. Time Warp80353 and Fury of the Aspects390386 are learned at49/48. Pet Axe Toss89766 is a learned Felguard action (family29, skill761, level17), not a Demonology player-spell grant. Hunter utility requires an actual existing/stabled pet, never an invented pet.', '']
    for profile in data['profiles']:
        rows = [r for r in data['spells'] if r['spec_id'] == profile['spec_id']]
        lines += [f"## {profile['spec_id']} — {profile['label']} / {profile['name']}", '', '| Spell | Levels | Category | Native effect/aura → child |', '|---|---|---|---|']
        for row in rows:
            facts = '; '.join(f"{e['Effect']}/{e['EffectAura']}" + (f"→{e['EffectTriggerSpell']}" if e['EffectTriggerSpell'] else '') for e in row['effects'])
            lines.append(f"| {row['name']} `{row['spell_id']}` | {row['min_level']}–{row['max_level']} | {row['category']} | {facts} |")
        lines.append('')
    lines += ['## Explicit boundaries', '', '- DPS/Healer Growl6795 and Provoke115546 stay excluded from autonomous taunts. Only tank profiles may taunt.', '- Purge/soothe/dispel eligibility uses native effect38 types and harmfulness, never spell-name assumptions. Absent selected spells are not granted.', '- Non-rotation/manual includes travel, portals, item creation, tracking, summons, forms, pet commands, situational utility and unsupported source mechanics. Census presence does not certify execution support.', '- Damage/heal rotational rows and passive talents are enumerated for completeness but are not redesigned by this task.', '']
    return '\n'.join(lines)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--database', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    data = census(args.database.resolve(strict=True))
    args.output.mkdir(parents=True, exist_ok=True)
    (args.output / 'census.json').write_text(json.dumps(data, indent=2) + '\n')
    (args.output / 'census.md').write_text(render(data))


if __name__ == '__main__':
    main()
