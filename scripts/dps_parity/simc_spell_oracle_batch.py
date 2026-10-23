#!/usr/bin/env python3
"""Generate top-five spell-channel oracles from the parity reference receipts."""
import argparse
import json
from pathlib import Path

from simc_spell_oracle import run_oracle, sha256

ENGINE = Path('/home/osso/Projects/world-of-osso/game-engine')
REFERENCES = ENGINE / 'data/diagnostics/dps-parity-2026-10-21'
OUTPUT = ENGINE / 'data/diagnostics/simc-spell-oracle-2026-10-23'
PRIORITY = [70, 269, 71, 577, 259, 1467, 63, 253]
# Explicit minimal public SimC cast drivers for reported child/proc channels.
DRIVERS = {
    70: {'final_verdict': ['templars_verdict'], 'dawnlight': ['avenging_wrath', 'wake_of_ashes', 'templars_verdict'], 'divine_arbiter': ['divine_storm,if=buff.divine_arbiter_verdict.up', 'templars_verdict'], 'crusading_strike': [], 'perfected_guillotine': ['use_items']},
    269: {'rising_sun_kick_damage': ['rising_sun_kick'], 'rushing_wind_kick_damage': ['rushing_wind_kick', 'blackout_kick'], 'dual_threat': []},
    71: {'execute_damage': ['execute'], 'slayers_strike': ['mortal_strike'], 'mortal_strike_bladestorm_unhinged': ['bladestorm'], 'bladestorm_mh': ['bladestorm']},
    577: {'burning_blades': ['immolation_aura', 'blade_dance', 'chaos_strike'], 'demonsurge': ['metamorphosis', 'eye_beam', 'blade_dance', 'chaos_strike'], 'death_sweep_first_blood_4': ['metamorphosis', 'death_sweep'], 'annihilation_damage_1': ['metamorphosis', 'annihilation']},
    1467: {'azure_sweep': ['azure_sweep', 'eternity_surge'], 'bombardments': ['deep_breath', 'disintegrate'], 'scintillation': ['disintegrate']},
    253: {'bloodshed': ['bestial_wrath'], 'melee': ['kill_command']},
    63: {'ignite': ['pyroblast'], 'intensifying_flame': ['fire_blast', 'pyroblast']},
    72: {'crushing_blow_mh': ['recklessness', 'raging_blow'], 'crushing_blow_oh': ['recklessness', 'raging_blow'], 'slayers_strike': ['rampage'], 'bladestorm_mh': ['bladestorm']},
    73: {'execute_damage': ['execute'], 'devastator': []},
    66: {'refining_fire': ['avengers_shield'], 'divine_guidance': ['consecration', 'judgment']},
    254: {'master_marksman': ['aimed_shot']},
    255: {'wildfire_bomb_damage': ['wildfire_bomb'], 'wildfire_bomb_bleed': ['wildfire_bomb'], 'raptor_swipe': ['raptor_strike'], 'strike_as_one': ['kill_command']},
    260: {'killing_spree_oh': ['killing_spree'], 'killing_spree_mh': ['killing_spree']},
    261: {'eviscerate_shadow_clone': ['shadow_dance', 'eviscerate'], 'eviscerate_bonus': ['eviscerate'], 'lingering_shadow': ['shadow_dance', 'eviscerate'], 'secret_technique_clones': ['secret_technique']},
    258: {'void_volley_damage': ['void_eruption', 'void_volley'], 'void_blast': ['void_eruption', 'mind_blast']},
    250: {'perfected_guillotine': ['use_items'], 'vampiric_strike': ['heart_strike']},
    251: {'obliterate_frost': ['pillar_of_frost', 'obliterate']},
    252: {'necrotic_bolt': ['death_coil'], 'death_order': ['dark_transformation', 'scourge_strike'], 'putrefy_st': ['putrefy'], 'sweeping_claws': ['dark_transformation']},
    262: {'lightning_bolt_overload': ['lightning_bolt']},
    263: {'crash_lightning_proc': ['crash_lightning', 'stormstrike'], 'fire_nova_explosion': ['flame_shock', 'fire_nova'], 'lightning_bolt_ti': ['lightning_bolt'], 'tempest': ['lightning_bolt']},
    62: {'prismatic_bolt': ['arcane_blast'], 'arcane_splinter': ['arcane_blast']},
    64: {'shatter_ice_lance': ['ice_lance'], 'frost_splinter': ['frostbolt']},
    265: {'blackened_soul': ['agony', 'unstable_affliction', 'shadow_bolt']},
    266: {'fel_firebolt': ['hand_of_guldan'], 'burning_cleave': ['summon_demonic_tyrant', 'hand_of_guldan'], 'isolated_implosion_aoe': ['implosion', 'hand_of_guldan'], 'greater_felbolt': ['summon_demonic_tyrant']},
    267: {'blackened_soul': ['immolate', 'chaos_bolt']},
    268: {'purified_spirit': ['purifying_brew'], 'aspect_of_harmony_damage': ['celestial_brew', 'tiger_palm'], 'harmonic_surge_damage_st_tiger_palm': ['celestial_brew', 'tiger_palm']},
    103: {'unseen_slash': ['rake', 'rip']},
    581: {'voidfall_meteor_damage': ['soul_cleave'], 'soul_cleave_damage': ['soul_cleave'], 'catastrophe': ['metamorphosis', 'soul_cleave'], 'world_killer_damage': ['soul_cleave'], 'meteor_shower_damage': ['soul_cleave']},
    1480: {'cull_damage': ['cull'], 'reap_damage': ['reap'], 'voidsurge': ['metamorphosis', 'void_ray'], 'reapers_toll_damage': ['reapers_toll']},
}
GENERATORS = {
    70: ['blade_of_justice', 'judgment'], 269: ['tiger_palm'], 71: ['overpower'],
    577: ['felblade'], 259: ['mutilate'], 1467: ['living_flame'], 253: ['barbed_shot'],
    72: ['bloodthirst'], 73: ['shield_slam'], 66: ['judgment', 'blessed_hammer'],
    255: ['kill_command'], 260: ['sinister_strike'], 261: ['backstab'],
    250: ['heart_strike'], 251: ['howling_blast'], 252: ['scourge_strike'],
    263: ['stormstrike'], 265: ['shadow_bolt'], 266: ['shadow_bolt'],
    267: ['incinerate'], 268: ['keg_smash'], 103: ['shred'], 581: ['fracture'], 1480: ['devour'],
}
NO_AUTO = {62, 63, 64, 258, 262, 265, 266, 267, 1467}


def top_channels(data):
    grouped = {}
    for row in data['actions']:
        if row['damage'] <= 0:
            continue
        key = row['name'], row['spell_id']
        entry = grouped.setdefault(key, {'name': row['name'], 'spell_id': row['spell_id'], 'damage': 0, 'share_pct': 0, 'actors': []})
        entry['damage'] += row['damage']
        entry['share_pct'] += row['share_pct']
        entry['actors'].append(row['actor'])
    return sorted(grouped.values(), key=lambda row: row['damage'], reverse=True)[:5]


def action_list(spec, channel):
    name = channel['name']
    driver = DRIVERS.get(spec, {}).get(name, [name])
    if name.startswith('auto_attack') or name == 'auto_shot':
        driver = []
    if name == 'font_of_venomous_rage':
        driver = ['use_items']
    startup = [] if spec in NO_AUTO else ['auto_attack']
    if spec in {253, 266, 265, 267, 252}:
        startup.insert(0, 'summon_pet')
    if spec == 103:
        startup.insert(0, 'cat_form')
    return list(dict.fromkeys(startup + driver + GENERATORS.get(spec, [])))


def reference_for(spec, references=REFERENCES):
    matches = sorted(references.rglob(f'{spec}.reference.single.normalized.json'))
    if not matches:
        raise ValueError(f'no normalized single-target reference for spec {spec}')
    reference = matches[0]
    raw = reference.with_name(f'{spec}.reference.single.json')
    proofs = [reference.with_name(f'{spec}.reference.single.proof.json'), reference.with_name(f'{spec}.single.proof.json')]
    proof_path = next((p for p in proofs if p.exists()), None)
    if proof_path is None:
        raise ValueError(f'no reference proof for {reference}')
    proof = json.loads(proof_path.read_text())
    profile = next((x for x in proof['command'] if x.endswith('.simc')), None)
    if not profile:
        raise ValueError(f'no SimC profile in {proof_path}')
    return reference, raw, proof_path, profile


# The pinned report aggregates these child formula actions under parent channels.
# Keep printed IDs unchanged and expose this relation in selection metadata.
FORMULA_ALIASES = {
    (269, 'fists_of_fury'): {'fists_of_fury_damage', 'fists_of_fury_damage_mid2_2pc'},
    (1467, 'fire_breath'): {'fire_breath_damage'},
}


def matching_formulas(spec, channel, events):
    aliases = FORMULA_ALIASES.get((spec, channel['name']), set())
    return [e for e in events if e['event'].endswith('_amount') and (
        (e['spell_id'] == channel['spell_id'] and e['action'] == channel['name'])
        or e['action'] in aliases)]


def generate_spec(spec, output=OUTPUT, references=REFERENCES):
    reference, raw, reference_proof, profile = reference_for(spec, references)
    channels = top_channels(json.loads(reference.read_text()))
    receipts = []
    for channel in channels:
        filename = f"{channel['name']}.json"
        # Preserve separate IDs if a report uses one action name for several spells.
        if sum(c['name'] == channel['name'] for c in channels) > 1:
            filename = f"{channel['name']}__{channel['spell_id']}.json"
        path = output / str(spec) / filename
        metadata = {'spec_id': spec, 'channel': channel, 'reference': str(raw), 'normalized_reference': str(reference), 'reference_sha256': sha256(raw), 'reference_proof': str(reference_proof), 'profile_policy': 'exact source profile from reference proof; MID1 where reference used MID1'}
        try:
            data = run_oracle(profile, action_list(spec, channel), path)
            observed = matching_formulas(spec, channel, data['events'])
            metadata['observed_formula_actions'] = sorted({(e['action'], e['spell_id']) for e in observed})
            metadata['status'] = 'observed' if observed else 'NOT_TRIGGERED_BY_MINIMAL_DRIVERS'
            metadata['matching_formula_records'] = len(observed)
            data['selection'] = metadata
            path.write_text(json.dumps(data, indent=2) + '\n')
        except RuntimeError as exc:
            metadata.update(status='SIMC_ACTION_INITIALIZATION_FAILED', error=str(exc))
            path.write_text(json.dumps({'selection': metadata, 'factors': None, 'proof': str(path.with_suffix('.proof.json'))}, indent=2) + '\n')
        receipts.append({'path': str(path), **metadata})
        print(f"{spec} {channel['name']}: {metadata['status']}", flush=True)
    manifest = output / str(spec) / 'manifest.json'
    manifest.write_text(json.dumps(receipts, indent=2) + '\n')
    write_index(output)
    return receipts


def write_index(output):
    text = ['# SimC spell oracle — 2026-10-23', '', 'Pinned 76f8118cf3f7f41dad20708e8328e1f4ab2deebc. One deterministic 30 s probe per selected channel; not DPS parity certification. Pet contributions with the same name/ID are combined for top-five selection. Receipt v5 Frost251 native-legality exclusion remains in force.', '', 'Each JSON retains all observed children/procs and links adjacent `.simc`, `.txt`, `.proof.json`, and console output. `NOT_TRIGGERED` means no formula evidence for the requested channel, not zero damage or an inferred formula. Initialization failures retain actual SimC errors.', '', '| Spec | Channel | Reference share | Formula records | Status |', '|---|---|---:|---:|---|']
    for manifest in sorted(output.glob('*/manifest.json'), key=lambda p: int(p.parent.name)):
        for row in json.loads(manifest.read_text()):
            channel = row['channel']
            path = Path(row['path']).relative_to(output)
            text.append(f"| {row['spec_id']} | [{channel['name']}]({path}) | {channel['share_pct']:.2f}% | {row.get('matching_formula_records', 0)} | {row['status']} |")
    (output / 'index.md').write_text('\n'.join(text) + '\n')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--spec', type=int, action='append')
    parser.add_argument('--references', type=Path, default=REFERENCES)
    parser.add_argument('--output', type=Path, default=OUTPUT)
    args = parser.parse_args()
    available = {int(p.name.split('.')[0]) for p in args.references.rglob('*.reference.single.normalized.json')}
    specs = args.spec or [s for s in PRIORITY if s in available] + sorted(available - set(PRIORITY))
    for spec in specs:
        generate_spec(spec, args.output, args.references)


if __name__ == '__main__':
    main()
