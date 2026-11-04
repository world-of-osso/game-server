#!/usr/bin/env python3
"""Convert a configured WCL pull to post-mitigation replay pressure."""
import argparse
import bisect
import collections
import json
import sqlite3
from pathlib import Path

DEFERRED_ENEMY_DAMAGE = {124255, 361029}
# Replay inputs: the tank's pull-specific Vampiric Blood timing (see healer-tank-replay/250.md).
REPLAYED_SELF_BUFFS = {55233}


def read_health_observations(events, actor_slots):
    observations = collections.defaultdict(list)
    for event in events:
        actor = event.get('sourceID' if event.get('resourceActor') == 1 else 'targetID')
        maximum = event.get('maxHitPoints', 0)
        health = event.get('hitPoints', 0)
        if actor in actor_slots and maximum > 0 and health <= maximum:
            observations[actor].append((int(event['fight_ms']), maximum))
    for actor in actor_slots:
        observations[actor].sort()
        if not observations[actor]:
            raise ValueError(f'no max HP observations for WCL actor {actor}')
    return observations


def max_health_at(observations, actor, time_ms):
    values = observations[actor]
    index = bisect.bisect_right(values, (time_ms, float('inf'))) - 1
    if index < 0:
        index = 0
    observed_ms, maximum = values[index]
    return maximum, observed_ms


def read_spell_metadata(db, spell):
    if spell == 1:
        return 1, 2  # WCL basic melee sentinel, not SpellInfo ID 1.
    if spell == 3:
        return 1, None  # WCL Falling sentinel, not SpellInfo ID 3.
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


def match_deferred_absorbs(events, healing):
    damage = collections.defaultdict(list)
    for index, event in enumerate(events):
        key = (event['targetID'], event['sourceID'], event.get('sourceInstance', 0),
               event['abilityGameID'], event['timestamp'])
        damage[key].append(index)
    deferred = collections.Counter()
    for absorb in healing:
        if absorb['type'] != 'absorbed' or absorb.get('abilityGameID') not in (115069, 357170):
            continue
        prefix = (absorb['targetID'], absorb['attackerID'], absorb.get('attackerInstance', 0),
                  absorb['extraAbilityGameID'])
        candidates = [i for ts in (absorb['timestamp'], absorb['timestamp'] + 1)
                      for i in damage[(*prefix, ts)]]
        if len(candidates) != 1:
            raise ValueError(f'cannot uniquely match deferred absorb: {absorb}')
        deferred[candidates[0]] += absorb['amount']
    for index, amount in deferred.items():
        if amount > events[index].get('absorbed', 0):
            raise ValueError(f'deferred absorb exceeds recorded absorb: {events[index]}')
    return deferred


def convert(events, healing, db, config):
    duration_ms = config['duration_ms']
    if duration_ms <= 0:
        raise ValueError('WCL replay duration must be positive')
    events = [event for event in events if 0 <= event['fight_ms'] < duration_ms]
    healing = [event for event in healing if 0 <= event['fight_ms'] < duration_ms]
    actor_slots = {actor: slot for slot, actor in enumerate(config['actors'])}
    if len(actor_slots) != 5 or len(config['party_specs']) != 5:
        raise ValueError('expected five distinct WCL actors and five party specs')
    observations = read_health_observations(events + healing, actor_slots)
    deferred_absorbs = match_deferred_absorbs(events, healing)
    hits = []
    observed_self = []
    excluded_self = 0
    for index, event in enumerate(events):
        target = event['targetID']
        if target not in actor_slots:
            continue
        source = event['sourceID']
        spell = event['abilityGameID']
        time_ms = int(event['fight_ms'])
        max_hp, observed_ms = max_health_at(observations, target, time_ms)
        deferred = source == target and spell in DEFERRED_ENEMY_DAMAGE
        unknown_priest_backlash = (source == -1 and spell == 32409
                                  and config['party_specs'][actor_slots[target]] in (256, 257, 258))
        if source == target or unknown_priest_backlash:
            observed_self.append({'time_ms': time_ms, 'slot': actor_slots[target],
                                  'spell_id': spell, 'amount': event.get('amount', 0),
                                  'absorbed': event.get('absorbed', 0), 'max_hp': max_hp})
            if not deferred:
                excluded_self += 1
                continue
        elif source in actor_slots:
            raise ValueError(f'unmapped friendly damage source {source}, spell {spell}')
        school, defense = read_spell_metadata(db, spell)
        health_after = event.get('hitPoints') if event.get('resourceActor') == 2 else None
        hits.append({'time_ms': time_ms, 'slot': actor_slots[target],
                     'source_actor': source, 'target_actor': target, 'spell_id': spell,
                     'school': school, 'defense': defense, 'hit_type': event['hitType'],
                     'periodic': event.get('tick', False), 'deferred': deferred,
                     'amount': event.get('amount', 0),
                     'absorbed': event.get('absorbed', 0) - deferred_absorbs[index],
                     'deferred_absorbed': deferred_absorbs[index], 'max_hp': max_hp,
                     'max_hp_observed_ms': observed_ms, 'health_after': health_after})
    if not hits or any(a['time_ms'] > b['time_ms'] for a, b in zip(hits, hits[1:])):
        raise ValueError('WCL damage events must be nonempty and ordered')
    initial_hp = [max_health_at(observations, actor, 0) for actor in config['actors']]
    return {'provenance': config['provenance'], 'actor_slots': actor_slots,
            'party_specs': config['party_specs'], 'initial_hp_observations': initial_hp,
            'bloodlust': config['bloodlust'], 'excluded_self': excluded_self,
            'observed_self': observed_self, 'hits': hits}


def convert_tank(events, healing, db, config):
    """Retain WCL source pressure; the native tank supplies recipient mitigation."""
    actors = config['actors']
    tank = actors[0]
    duration = config['duration_ms']
    observations = read_health_observations(events + healing, {tank: 0})
    hits = []
    excluded = collections.Counter()
    for event in events:
        time_ms = int(event['fight_ms'])
        if not 0 <= time_ms < duration or event.get('targetID') != tank:
            continue
        source = event['sourceID']
        if source == tank:
            excluded['self_events'] += 1
            continue
        raw = event.get('unmitigatedAmount')
        amount = event.get('amount', 0)
        absorbed = event.get('absorbed', 0)
        if source in actors:
            excluded['friendly_events'] += 1
            excluded['friendly_observed_pressure'] += amount + absorbed
            if raw is not None:
                excluded['known_raw'] += raw
            continue
        if raw is None and amount + absorbed > 0:
            excluded['missing_raw_events'] += 1
            excluded['missing_raw_observed_pressure'] += amount + absorbed
            continue
        if raw is not None and raw != amount + absorbed + event.get('mitigated', 0):
            excluded['inconsistent_events'] += 1
            excluded['known_raw'] += raw
            continue
        school, defense = read_spell_metadata(db, event['abilityGameID'])
        maximum, observed_ms = max_health_at(observations, tank, time_ms)
        hits.append({'time_ms': time_ms, 'slot': 0, 'source_actor': source,
                     'source_instance': event.get('sourceInstance', 0), 'target_actor': tank,
                     'spell_id': event['abilityGameID'], 'school': school, 'defense': defense,
                     'hit_type': event['hitType'], 'periodic': event.get('tick', False),
                     'raw': raw, 'amount': amount, 'absorbed': absorbed, 'max_hp': maximum,
                     'max_hp_observed_ms': observed_ms,
                     'health_after': event.get('hitPoints') if event.get('resourceActor') == 2 else None})
    external_heals = []
    for event in healing:
        time_ms = int(event['fight_ms'])
        if (0 <= time_ms < duration and event.get('type') == 'heal'
                and event.get('targetID') == tank and event.get('sourceID') != tank):
            maximum, _ = max_health_at(observations, tank, time_ms)
            external_heals.append({'time_ms': time_ms, 'source_actor': event['sourceID'],
                                   'spell_id': event['abilityGameID'],
                                   'amount': event.get('amount', 0), 'max_hp': maximum})
    included_raw = sum(hit['raw'] or 0 for hit in hits)
    actor_slots = {actor: slot for slot, actor in enumerate(actors)}
    for pet, owner in config.get('pet_owners', {}).items():
        if owner in actor_slots:
            actor_slots[int(pet)] = actor_slots[owner]
    known_raw_total = included_raw + excluded['known_raw']
    excluded_raw_share = excluded['known_raw'] / known_raw_total if known_raw_total else None
    return {'mode': 'tank_raw', 'provenance': config['provenance'],
            'excluded_known_raw_share': excluded_raw_share,
            'actor_slots': actor_slots,
            'party_specs': config['party_specs'], 'bloodlust': config['bloodlust'],
            'excluded_self': excluded['self_events'], 'excluded': dict(excluded),
            'included_raw': included_raw, 'hits': hits, 'external_heals': external_heals}


def opening_auras(combatants, buffs, actors):
    slots = {actor: slot for slot, actor in enumerate(actors)}
    initial = {}
    for combatant in combatants:
        target = combatant['sourceID']
        if target not in slots:
            continue
        for aura in combatant['auras']:
            key = (aura['source'], target, aura['ability'])
            initial[key] = {'source_actor': key[0], 'target_actor': target,
                            'spell_id': key[2], 'stacks': aura['stacks'],
                            'evidence': 'combatant_info'}
    first = {}
    for event in buffs:
        if event['fight_ms'] < 0 or event.get('targetID') not in slots:
            continue
        key = (event['sourceID'], event['targetID'], event['abilityGameID'])
        first.setdefault(key, event)
    for key, event in first.items():
        if key in initial or event['type'] not in ('removebuff', 'refreshbuff', 'removebuffstack'):
            continue
        stacks = event.get('stack', 0) + 1 if event['type'] == 'removebuffstack' else 1
        initial[key] = {'source_actor': key[0], 'target_actor': key[1], 'spell_id': key[2],
                        'stacks': stacks, 'evidence': 'first_' + event['type']}
    for event in buffs:
        key = (event.get('sourceID'), event.get('targetID'), event.get('abilityGameID'))
        if key in initial and event['type'] == 'removebuff' and event['fight_ms'] >= 0:
            initial[key].setdefault('remove_ms', int(event['fight_ms']))
    return list(initial.values())


def read_absorb_metadata(db, spell):
    rows = db.execute('SELECT EffectAura, EffectAuraPeriod FROM spell_effect '
                      'WHERE SpellID=? AND DifficultyID=0', (spell,)).fetchall()
    return any(aura == 69 for aura, _ in rows), all(period == 0 for _, period in rows)


def has_exhausting_partial_hit(absorbs, damage, tank, removal):
    return any(
        abs(absorb['fight_ms'] - removal) <= 1 and any(
            event.get('targetID') == tank and event.get('sourceID') == absorb['attackerID']
            and event.get('sourceInstance', 0) == absorb.get('attackerInstance', 0)
            and event.get('abilityGameID') == absorb['extraAbilityGameID']
            and abs(event['timestamp'] - absorb['timestamp']) <= 1
            and event.get('amount', 0) > 0 and event.get('absorbed', 0) >= absorb['amount']
            for event in damage)
        for absorb in absorbs)


def infer_exhausted_opening_absorbs(auras, damage, healing, config, db, buffs):
    """Static external shields retain consumed points plus their observed removal remainder."""
    tank = config['actors'][0]
    observations = read_health_observations(damage + healing, {tank: 0})
    result = []
    for aura in auras:
        entry = dict(aura)
        result.append(entry)
        removal = aura.get('remove_ms')
        owned = aura['source_actor'] == tank
        if (aura['target_actor'] != tank or removal is None
                or aura['evidence'] != 'first_removebuff' or owned and aura['spell_id'] != 1235568):
            continue
        shield, static = read_absorb_metadata(db, aura['spell_id'])
        if not shield or not static:
            continue
        removed = next((event for event in buffs if event['type'] == 'removebuff'
                        and event.get('sourceID') == aura['source_actor'] and event.get('targetID') == tank
                        and event.get('abilityGameID') == aura['spell_id']
                        and int(event['fight_ms']) == removal), None)
        if removed is None or 'absorb' not in removed:
            continue
        remaining = removed['absorb']
        if remaining < 0:
            raise ValueError('negative remaining opening shield')
        absorbs = [event for event in healing if event['type'] == 'absorbed'
                   and event.get('sourceID') == aura['source_actor']
                   and event.get('targetID') == tank and event.get('abilityGameID') == aura['spell_id']
                   and 0 <= event['fight_ms'] <= removal]
        if owned and not has_exhausting_partial_hit(absorbs, damage, tank, removal):
            continue
        total = sum(event['amount'] for event in absorbs) + remaining
        if total <= 0:
            if not owned and removal == 0 and remaining == 0:
                entry['skip_reason'] = 'observed_empty_shield_removed_at_pull_start'
            continue
        maximum, _ = max_health_at(observations, tank, 0)
        entry['absorb_balance'] = {'amount': total, 'max_hp': maximum,
                                   'remaining_on_removal': remaining,
                                   'evidence': 'INFERRED_WCL_static_shield_receipts_and_remainder'}
    return result


def logged_external_shields(buffs, damage, healing, db, config):
    tank = config['actors'][0]
    observations = read_health_observations(damage + healing, {tank: 0})
    result = []
    metadata = {}
    for event in buffs:
        time_ms = int(event['fight_ms'])
        if (event['type'] not in ('applybuff', 'refreshbuff') or 'absorb' not in event
                or event.get('sourceID') == tank or event.get('targetID') != tank
                or not 0 <= time_ms < config['duration_ms']):
            continue
        spell = event['abilityGameID']
        if spell not in metadata:
            metadata[spell] = read_absorb_metadata(db, spell)
        if not metadata[spell][0]:
            continue
        if event['absorb'] < 0:
            raise ValueError('negative logged shield grant')
        maximum, _ = max_health_at(observations, tank, time_ms)
        result.append({'time_ms': time_ms, 'source_actor': event['sourceID'],
                       'target_actor': tank, 'spell_id': spell, 'amount': event['absorb'],
                       'max_hp': maximum, 'evidence': 'WCL_' + event['type'] + '_absorb'})
    return sorted(result, key=lambda event: event['time_ms'])


def replayed_casts(buffs, actors, duration_ms):
    slots = {actor: slot for slot, actor in enumerate(actors)}
    casts = [{'time_ms': int(event['fight_ms']), 'slot': slots[event['targetID']],
              'spell_id': event['abilityGameID'], 'evidence': 'applybuff'}
             for event in buffs
             if event['type'] == 'applybuff' and event['abilityGameID'] in REPLAYED_SELF_BUFFS
             and event.get('sourceID') == event.get('targetID') and event['targetID'] in slots
             and 0 <= event['fight_ms'] < duration_ms]
    return sorted(casts, key=lambda cast: cast['time_ms'])


def read_jsonl(path):
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('events', type=Path)
    parser.add_argument('healing', type=Path)
    parser.add_argument('world_db', type=Path)
    parser.add_argument('config', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--combatants', type=Path, required=True)
    parser.add_argument('--buffs', type=Path, required=True)
    parser.add_argument('--mode', choices=('healer', 'tank'), default='healer')
    args = parser.parse_args()
    with sqlite3.connect(f'file:{args.world_db}?mode=ro', uri=True) as db:
        config = json.loads(args.config.read_text())
        converter = convert_tank if args.mode == 'tank' else convert
        result = converter(read_jsonl(args.events), read_jsonl(args.healing), db, config)
        buffs = read_jsonl(args.buffs)
        result['opening_auras'] = opening_auras(read_jsonl(args.combatants), buffs,
                                               config['actors'])
        result['replayed_casts'] = replayed_casts(buffs, config['actors'], config['duration_ms'])
        if args.mode == 'tank':
            result['opening_auras'] = infer_exhausted_opening_absorbs(
                result['opening_auras'], read_jsonl(args.events), read_jsonl(args.healing), config, db, buffs)
            result['empty_opening_shields'] = [aura for aura in result['opening_auras'] if 'skip_reason' in aura]
            result['opening_auras'] = [aura for aura in result['opening_auras'] if 'skip_reason' not in aura]
            result['external_shields'] = logged_external_shields(
                buffs, read_jsonl(args.events), read_jsonl(args.healing), db, config)
    args.output.write_text(json.dumps(result, separators=(',', ':')) + '\n')


if __name__ == '__main__':
    main()
