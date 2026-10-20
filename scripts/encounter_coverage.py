#!/usr/bin/env python3
"""Read-only census of every hosted DungeonEncounter / JournalEncounter.

Structural admission is not behavioral proof. No names create creature bindings;
ambiguous journal display joins remain gaps. No world database is modified.
"""
import argparse
import csv
import hashlib
import json
import sqlite3
from collections import defaultdict
from contextlib import closing
from pathlib import Path

from raid_encounters import read_journal
from encounter_order import load_order

EVENTS = {0, 4, 6, 7, 8, 11, 17, 19, 20, 23, 25, 35, 40, 54, 55, 56, 57, 58, 61, 65}
ACTIONS = {1, 2, 6, 11, 12, 17, 22, 29, 33, 37, 41, 53, 54, 55, 59, 64, 65, 66, 80, 81, 82, 83, 107, 152}
TARGETS = {0, 1, 2, 5, 7, 8, 12, 16, 19}
# TC a352b1fa Murder Row / Kings' Rest / Ragefire / Deadmines headers and instances identify these primaries,
# even where no placement/model join is available. These census facts do not bind runtime credit.
CPP_IDENTITIES = {(2813,3101): [234648], (2813,3102): [234649],
                  (2813,3103): [234647], (2813,3105): [237415], (1762,2139): [135322],
                  (36,2976): [47162], (36,2981): [47162],
                  (36,2977): [47296], (36,2982): [47296],
                  (36,2975): [43778], (36,2980): [43778],
                  (36,2974): [47626], (36,2979): [47626],
                  (36,2973): [47739], (36,2978): [47739], (36,1081): [49541],
                  (389,1445): [61463]}
# Header e3beb3f1a16472f10655148b1cd6ffd07187ff58aa0b6fee3ebfc584bcc9944a;
# instance 156a63da23e1ac0f725ba9d1b7397a024b61077537816e279cef4dba54f749af.
DEADMINES_SOURCE_GAPS = [
    'pinned TC has no Deadmines boss AI; primary templates have no C++/SmartAI script; no invented schedule',
    'source encounter ordinals0-5 differ from DB2 difficulty-specific bits; current map-entry binding cannot express Normal/Heroic bit differences',
    'source doors are identified but engage/kill/credit/loot/lockout/native acceptance unproved',
]
PARTIAL_PORTS = {'boss_kystia_manaheart': [474365,1230298,1264095,1264098,474240,474183,1223906,1217989,1265412],
                 'boss_the_golden_serpent': [265773,265781,265910,265923],
                 'boss_dark_shaman_koranthal': [119928,119300,119971,119973],
                 'boss_lava_guard_gordoth': [119999,120024,50420],
                 'boss_slagmaw': [119434,120384,119424,119425,119426,119428],
                 'boss_watchkeeper_gargolmar': [30641,34645,30642,22857],
                 'boss_omor_the_unscarred': [30686,30695,30707,35642,31901],
                 'boss_vazruden_the_herald': [30717,30698,30694,30700,18430,20538,30691,32491,
                                              33793,33794,34653,36920,30926,36921,39427],
                 'boss_vazruden': [5301,30693,19130,40392],
                 'boss_the_maker': [40059],
                 'boss_noxxion': [21687,22916,21708,21707],
                 'boss_celebras_the_cursed': [21807,12747,21968],
                 'boss_landslide': [18670,5568,21808],
                 'boss_princess_theradras': [21909,21832,3391,21869]}
PARTIAL_GAPS = {
    'boss_slagmaw': [
        'source five Lava Spits/Submerge/nonrepeat teleport/emerge and model-reach boundary supported; server-side spells/model geometry are separate reviewed-copy imports',
        'pinned NPC61463/BOSS2 maps encounter1445 despite Journal preview38391 versus live TDB model42247; no display replacement or inferred identity from appearance',
        'primary binding prepared for lead review; accessory61800/encounter-frame engage, loot/lockout/native/full-instance proof remain open'],
    'boss_lava_guard_gordoth': [
        'source already-started relocation,first7.3s Ground/13.3s Seismic and one-shot below31-percent enrage/30s evade-respawn supported; Seismic uses only source0x2 cooldown flag',
        'repeat12.1s/36.3s callbacks coded but moving-target/native proof unproved; source jump/landing/vial state/researcher self-death supported; landing home/facing and ordinary-player AT7899 rotated-box entry/same-copy lookup/OnlyOnce supported; GM/cage task/animkit/native jump gaps; map389 navigation absent',
        'canonical primary credit binding, loot/lockout/native/full-instance proof remain open'],
    'boss_dark_shaman_koranthal': [
        'source appearance,6s/7.3s Twisted and20.5s/47.2s Storm/reschedule15.7s supported; dummy119973 effect value119965 forwards missile destination to119964 damage',
        'source death/30s evade-respawn fixtures supported; canonical primary credit binding, loot/lockout, native visuals and full instance remain unproved'],
    'boss_kystia_manaheart': [
        'infusion completion blocked: local15s Light/20s Destabilized; exact474238 flags do not bypass caster auras',
        'mirror Normal template/callbacks/retirement supported on private copy; other difficulties and Felstorm damage unproved',
        'Fel Spray area geometry/damage, root/visual kits, door/boundary and source-empty loot unproved'],
    'boss_the_golden_serpent': [
        'TC script schedule (partial; Retail 12.1 timeline inputs unavailable); Retail alerts disabled',
        'source intro supported: group1256 spawns on last TempleEvent death, gravity-disabled MovePath13532200/01/02, emerge removal and hostile landing; group1255 withheld until Serpentine Seal gossip summons Zul (SerpentBossEvent +10s spawn); Shadow-Borne temple trash schedules and Gust Slash orbiting AT supported; Minion of Zul appearance/Bound by Shadow/fixate supported; pool GO state/collision supported; native state-to-animation mapping unported; fly/hover anim state unproved',
        'Molten Gold source static sphere/Player enter-exit and periodic damage supported; native area visuals unproved',
        'delayed follow/absorption/announcement supported: real-bake,3D reach,subyard repath,root/cast pause and4s retirement fixtures; source near-point collision/floor and full motion-priority equivalence unproved',
        'source-empty loot unproved',
        'ordinary balance, full instance and both-skin native acceptance unproved'],
    'boss_watchkeeper_gargolmar': [
        'source timed10-15min idle taunt, Mortal Wound30641 victim, Surge34645 farthest threat, incoming-below40 Request Heal30642 once and below20 Retaliation22857 then30s repeats; reset re-arms both',
        'ordinary/native fight, loot and persistence unproved'],
    'boss_omor_the_unscarred': [
        'source Shadow Bolt30686, Treacherous Aura30695, 30% hound summon30707 on cast, Orbital Strike35642 and Demonic Shield31901 supported',
        'ordinary/native fight, loot and persistence unproved'],
    'boss_vazruden_the_herald': [
        'source intro/flight/hover, descent and fall, Normal/Heroic ground parents and ground/flying Liquid Fire children supported',
        'open: GO_NOT_SELECTABLE_FLAG_UNMODELLED (Fel Iron chest), NPC_ONESHOT_EMOTE_UNIMPLEMENTED, CREATURE_ENTRY_RESPAWN_RESTORE_UNIMPLEMENTED, TARGET32 XY/collision/hover gaps',
        'native presentation, Heroic balance, loot and persistence unproved'],
    'boss_vazruden': [
        'source5301 appearance, 5s aggro speech, Revenge19130/40392, incoming-below40 Call Nazan30693 and unfiltered slay speech supported',
        'ordinary/native fight, loot and persistence unproved'],
    'boss_the_maker': [
        'source aggro speech, Heroic40059 beaker 5-15s/10-20s, player-only slay speech and death credit supported',
        'open: MAKER_NORMAL_BEAKER_30925_ABSENT_70448 (Normal casts nothing), MAKER_DOMINATION_CHARM_UNIMPLEMENTED, BLOOD_FURNACE_DOOR_DATA_UNIMPLEMENTED',
        'ordinary/native fight, loot and persistence unproved'],
    'boss_noxxion': [
        'source Toxic Volley21687, Uppercut22916 and Summon Dummy21708 passive/uninteractible window with triggered21707 five13456 spawns supported',
        'ordinary/native clear, knockback movement, loot and persistence unproved'],
    'boss_celebras_the_cursed': [
        'source Wrath21807, Entangling Roots12747 and Corrupt Forces21968 summon13743, death summon13716 supported',
        'ordinary/native fight, loot and persistence unproved'],
    'boss_landslide': [
        'source Knock Away18670, Trample5568 and below50 Landslide21808 then30-40s repeats; reset re-arms once',
        'ordinary/native fight, loot and persistence unproved'],
    'boss_princess_theradras': [
        'source Dust Field21909, Boulder21832, Thrash3391 dispatch and Repulsive Gaze21869; death summon Zaetar12238',
        'Thrash extra-attacks effect19 unsupported; ordinary/native fight, loot, persistence and spirit quest/gossip unproved'],
}
PORTS = {'boss_hogger': [86604, 86620, 86736],
         'boss_lord_overheat': [12466, 86633, 86636],
         'boss_randolph_moloch': [86726, 86729, 55964, 55966]}


def catalog_records(encounters, journals, journal_maps, hosted):
    records = {(e['MapID'], e['ID']): dict(map_id=e['MapID'], encounter_id=e['ID'],
                name=e['Name_lang'], bit=e['Bit'], difficulty=e['DifficultyID'], journal_ids=[])
               for e in encounters if e['MapID'] in hosted}
    for journal in journals:
        map_id = journal_maps.get(journal[5])
        if map_id not in hosted:
            continue
        encounter_id = journal[6] & 0xFFFF
        key = (map_id, encounter_id) if encounter_id else (map_id, -journal[4])
        record = records.setdefault(key, dict(map_id=map_id, encounter_id=encounter_id,
                  name=journal[0], bit=None, difficulty=None, journal_ids=[]))
        record['journal_ids'].append(journal[4])
    return sorted(records.values(), key=lambda r: (r['map_id'], r['encounter_id'], r['journal_ids']))


def wailing_boss_state_action(row, map_id):
    return (map_id == 43 and row['action_type'] == 34
            and row['action_param3'] == 1
            and 0 <= row['action_param1'] < 8
            and 0 <= row['action_param2'] < 4)


def row_gaps(row, map_id=None):
    gaps = []
    for field, kinds in [('event_type', EVENTS), ('action_type', ACTIONS), ('target_type', TARGETS)]:
        supported = row[field] in kinds
        if field == 'action_type':
            supported = supported or wailing_boss_state_action(row, map_id)
        if not supported:
            gaps.append(f"{field.split('_')[0]} {row[field]}")
    if row['event_flags'] & ~0x101:
        gaps.append(f"event flags {row['event_flags']}")
    if row['action_type'] == 11:
        if row['target_type'] == 0:
            gaps.append('cast has no unit target')
        if row['action_param2'] & ~0x62:
            gaps.append(f"cast flags {row['action_param2']}")
    if row['event_type'] == 8 and row['event_param2']:
        gaps.append('spell school condition')
    return gaps


def resolve_actors(map_id, journal_ids, actors, candidates):
    entries, gaps = set(), []
    for actor in actors:
        if actor[3] & 0xFFFF not in journal_ids:
            continue
        display = actor[4] & 0xFFFFFFFF
        matches = candidates.get((map_id, display), [])
        if len(matches) == 1:
            entries.add(matches[0])
        else:
            gaps.append(f'journal actor {actor[2]}: display {display} has {len(matches)} same-map candidates')
    return sorted(entries), gaps


def index_rows(connection, query, key):
    indexed = defaultdict(list)
    for row in connection.execute(query):
        indexed[row[key]].append(dict(row))
    return indexed


def collect(connection, journals, actors, journal_maps, compat, order):
    connection.row_factory = sqlite3.Row
    maps = {r['ID']: dict(r) for r in connection.execute("SELECT * FROM map WHERE InstanceType IN (1,2) AND lower(Directory) NOT LIKE '%garrison%' AND EXISTS (SELECT 1 FROM map_difficulty d WHERE d.MapID=map.ID)")}
    records = catalog_records([dict(r) for r in connection.execute('SELECT * FROM dungeon_encounter')], journals, journal_maps, set(maps))
    candidates = defaultdict(set)
    for r in connection.execute('SELECT DISTINCT c.map,m.CreatureDisplayID,c.id FROM tdb_creature c JOIN tdb_creature_template_model m ON m.CreatureID=c.id'):
        candidates[(r[0], r[1])].add(r[2])
    candidates = {key: sorted(value) for key, value in candidates.items()}
    bindings = index_rows(connection, 'SELECT * FROM raid_encounter_creature', 'map_id')
    for map_id, entry, encounter_id, ordinal in compat:
        bindings[map_id].append(dict(creature_entry=entry, encounter_id=encounter_id, bit=ordinal, compatibility=True))
    templates = {r['entry']: dict(r) for r in connection.execute('SELECT * FROM content_creature_template')}
    source = {r['entry']: dict(r) for r in connection.execute('SELECT entry,name,AIName,ScriptName FROM tdb_creature_template')}
    spawns = index_rows(connection, 'SELECT map,id1,spawnDifficulties FROM content_creature', 'id1')
    scripts = index_rows(connection, 'SELECT * FROM tdb_smart_scripts WHERE source_type=0 AND entryorguid>0', 'entryorguid')
    difficulties = index_rows(connection, 'SELECT * FROM content_creature_template_difficulty', 'entry')
    locks = index_rows(connection, 'SELECT MapID,DifficultyID,ResetInterval FROM map_difficulty', 'MapID')
    loot_counts = dict(connection.execute('SELECT Entry,count(*) FROM content_creature_loot_template GROUP BY Entry'))
    conditions = index_rows(connection, 'SELECT SourceEntry,SourceGroup FROM tdb_conditions WHERE SourceTypeOrReferenceId=22', 'SourceEntry')
    for record in records:
        mid = record['map_id']
        record['instance'] = maps[mid]['MapName_lang']
        record['priority'] = order[mid]
        entries, gaps = resolve_actors(mid, record['journal_ids'], actors, candidates)
        bound = [b for b in bindings[mid] if b['encounter_id'] == record['encounter_id'] or
                 (record['bit'] is not None and b['bit'] == record['bit'] and not b.get('compatibility'))]
        entries = sorted(set(entries) | {b['creature_entry'] for b in bound} |
                         set(CPP_IDENTITIES.get((mid,record['encounter_id']), [])))
        record['binding_gaps'] = gaps
        record['actors'] = []
        for entry in entries:
            template = templates.get(entry)
            origin = source.get(entry, {})
            rows = scripts[entry]
            loaded = template is not None and template['AIName'] == 'SmartAI'
            script = template['ScriptName'] if template else origin.get('ScriptName', '')
            supported = sorted({r['action_param1'] for r in rows if r['action_type'] == 11 and loaded and not row_gaps(r, mid)})
            authored = sorted({r['action_param1'] for r in rows if r['action_type'] in (11,85,86)})
            row_issues = [dict(row=r['id'], reasons=row_gaps(r, mid)) for r in rows if row_gaps(r, mid)]
            if conditions[entry]:
                row_issues.append(dict(row='conditions', reasons=['SmartScript conditions not interpreted']))
            has_binding = entry in {b['creature_entry'] for b in bound}
            if script in PARTIAL_PORTS:
                row_issues.append(dict(row='TC callbacks', reasons=PARTIAL_GAPS[script]))
            if mid == 36 and (mid, record['encounter_id']) in CPP_IDENTITIES:
                row_issues.append(dict(row='TC instance source', reasons=DEADMINES_SOURCE_GAPS))
            actor_spawns = [r for r in spawns[entry] if r['map'] == mid]
            loot = [dict(difficulty=d['DifficultyID'], lootid=d['lootid'], rows=loot_counts.get(d['lootid'],0),
                         gold=[d['mingold'], d['maxgold']]) for d in difficulties[entry]]
            if template:
                loot.insert(0, dict(difficulty='base',lootid=template['lootid'], rows=loot_counts.get(template['lootid'],0), gold=[template['mingold'],template['maxgold']]))
            record['actors'].append(dict(entry=entry, name=origin.get('name', template['name'] if template else ''),
                spawned=len(actor_spawns), spawn_difficulties=sorted({str(s['spawnDifficulties']) for s in actor_spawns}),
                template=template is not None, source_ai=origin.get('AIName',''), source_cpp=origin.get('ScriptName',''),
                runtime_ai=('partial TC C++ port' if has_binding else 'partial TC C++ port blocked by missing binding') if script in PARTIAL_PORTS else
                    ('ported TC C++' if script in PORTS else ('SmartAI' if loaded else 'melee-only lifecycle' if has_binding else 'no registered encounter AI')),
                admitted_casts=PARTIAL_PORTS.get(script,PORTS.get(script,supported)) if has_binding or loaded else [], authored_smart_casts=authored,
                ai_gaps=row_issues, cpp_gap=bool(script and script not in PORTS and script not in PARTIAL_PORTS),
                encounter_start_end='wired; unproved for this boss' if has_binding and actor_spawns else 'blocked: no binding/spawn',
                kill_credit='wired; unproved for this boss' if has_binding and actor_spawns else 'blocked: no binding/spawn',
                lockout='wired; unproved for this boss' if has_binding else 'blocked: no binding',
                loot=loot, proof='structural inspection only'))
        record['lockout_difficulties'] = [dict(difficulty=r['DifficultyID'], reset_interval=r['ResetInterval']) for r in locks[mid]]
        if not entries:
            record['binding_gaps'].append('no authenticated creature identity; no lifecycle/credit/loot certification')
    return records


def render(records, database, revision):
    instance_order = {r['map_id']: r for r in records}
    priorities = ['## Work order (user-approved, not measured popularity)', '',
                  'Tier1: locally related Midnight Season2 dungeons/current raid. Tier2: Normal leveling LFG dungeons by ContentTuning.MinLevelSquish (12.1 LFGDungeons no longer has MinLevel). Tier3: remaining instances, newest Map.ExpansionID first. Missing local tier relations/tuning stay in Tier3. This prioritizes current content and the new-character path without inventing production usage statistics or changing the Mythic+ catalog.', '',
                  'Local DisplaySeason37 explicitly names Midnight Season2. MythicPlusSeasonTrackedMap relates challenge IDs to DisplaySeason (not directly to MythicPlusSeason.ID). JournalTier505 is Current Season; AvailabilityCondition156363 distinguishes Season2 entries. Both dungeon relations must exist; raids require JournalTierXInstance→JournalInstance→Map. No server season activation is inferred.', '',
                  '|Rank|Tier|Map / instance|Local row IDs / rationale|', '|---:|---:|---|---|']
    for rank, r in enumerate(sorted(instance_order.values(), key=lambda r: r['priority']['sort']), 1):
        p = r['priority']
        refs = ', '.join(f'{k}={v}' for k,v in p.items() if k not in ('tier','sort','reason'))
        priorities.append(f"|{rank}|{p['tier']}|{r['map_id']} {r['instance']}|{refs}; {p['reason']}|")
    lines = ['# Hosted encounter coverage', '', 'Verified: 2026-10-13. **Census, not completion certification.**', '',
             f'Server revision: `{revision}`. Input: `{database}` (read-only).',
             'Sources: local TDB1210, DB2 12.1.0.69933 JournalEncounter/Creature and Map/DungeonEncounter; work-order relations: locally extracted DisplaySeason4279827, MythicPlusSeasonTrackedMap4521365, JournalTier1349056, JournalTierXInstance1285009, MapChallengeMode801709 and LFGDungeons1361033. TC a352b1fa Murder Row/KingsRest/Ragefire/Deadmines headers and instance scripts supply primary actor identities; pinned hashes are in scripts/encounter_cpp_bindings.py and scripts/encounter_coverage.py.', '',
             'Every DungeonEncounter difficulty record and Journal-only encounter on an admissible party/raid map is retained. Garrisons excluded. A missing spawn is not a hosted fight. Display joins identify candidate actors; only imported bindings/compatibility data authorize kill credit. Multi-actor completion remains unproved.', '',
             '“Admitted casts” means current interpreter vocabulary or an existing Stockade Rust port, not correct damage, targeting, phases or complete boss mechanics. Spell effects, conditions, triggered chains and source-9 lists require behavioral certification. “Wired” is code reachability, not passing engage→kill→loot proof. Spawn counts are database placements, not observed live entities. Loot rows/gold are bindings, not awarded loot proof.', '',
             *priorities, '',
             f"Census: {len(records)} encounter records; {len({r['map_id'] for r in records})} maps; {sum(len(r['actors']) for r in records)} actor projections (difficulty duplicates retained).", '',
             '## Per-boss census', '', '|Map / instance|Encounter / journal|Boss / spawn count|AI source → runtime|Admitted cast IDs / gaps|START/END; credit; lockout|Loot bindings / proof|', '|---|---|---|---|---|---|---|']
    for r in sorted(records,key=lambda r:(r['priority']['sort'],r['encounter_id'],r['journal_ids'])):
        base = [f"{r['map_id']} {r['instance']}", f"{r['encounter_id']} (difficulty {r['difficulty']}, bit {r['bit']}); J:{','.join(map(str,r['journal_ids']))}"]
        if not r['actors']:
            lines.append('|'+ '|'.join(base+[r['name']+' / unresolved','unknown','; '.join(r['binding_gaps']),'blocked; blocked; blocked','unknown; unproved'])+'|')
        for a in r['actors']:
            loot = '; '.join(f"{d['difficulty']}:{d['lootid']} ({d['rows']} rows, gold {d['gold'][0]}–{d['gold'][1]})" for d in a['loot']) or 'no template'
            gap = '; '.join(f"row {g['row']}: {', '.join(g['reasons'])}" for g in a['ai_gaps'])
            if a['cpp_gap']:
                gap += '; TC C++ script unported'
            fields = base+[f"{a['name']} [{a['entry']}] / {a['spawned']}", f"{a['source_ai'] or a['source_cpp'] or 'none'} → {a['runtime_ai']}",
                ','.join(map(str,a['admitted_casts']))+' / '+(gap or 'no row-vocabulary gap; mechanics/phases unproved'),
                '; '.join(a[k] for k in ('encounter_start_end','kill_credit','lockout')),loot+' / '+a['proof']]
            lines.append('|'+ '|'.join(s.replace('|','/') for s in fields)+'|')
    return '\n'.join(lines)+'\n'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--db',type=Path,required=True)
    parser.add_argument('--journal',type=Path,required=True)
    parser.add_argument('--creatures',type=Path,required=True)
    parser.add_argument('--journal-instances',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--json',type=Path,required=True)
    parser.add_argument('--revision',required=True)
    parser.add_argument('--order-db2',type=Path,required=True)
    args = parser.parse_args()
    journals=read_journal(args.journal,0xB88D11CC,13,{2:2})
    actors=read_journal(args.creatures,0x81A3C769,9)
    with args.journal_instances.open() as file:
        journal_maps={int(r['ID']):int(r['MapID']) for r in csv.DictReader(file)}
    compat_path=Path(__file__).parent.parent/'crates/server/src/scripted_ai/encounter_compat.csv'
    compat=[tuple(map(int,r.split(','))) for r in compat_path.read_text().splitlines() if r and not r.startswith('#')]
    with closing(sqlite3.connect(args.db.resolve().as_uri()+'?mode=ro',uri=True)) as conn:
        conn.row_factory=sqlite3.Row
        conn.execute('BEGIN')
        order=load_order(conn,args.order_db2,journal_maps)
        records=collect(conn,journals,actors,journal_maps,compat,order)
    hashes={str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in [args.journal,args.creatures,args.journal_instances,compat_path]}
    hashes.update({str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in args.order_db2.glob('*.db2')})
    args.json.write_text(json.dumps(dict(revision=args.revision,source_hashes=hashes,records=records),indent=2)+'\n')
    args.output.write_text(render(records,args.db,args.revision))
    print(f'{len(records)} records, {len({r["map_id"] for r in records})} maps; wrote {args.output}')


if __name__ == '__main__':
    main()
