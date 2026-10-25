//! Actual combat log accounting; no inferred hits or fabricated stat contributions.
use bevy::prelude::*;
use serde_json::{Value, json};
use shared::protocol::{CombatLogEvent, CombatLogKind};
use std::collections::BTreeMap;

pub(super) fn stats(world: &World, actor: Entity) -> Value {
    let sheet = world.get::<crate::stats::CharacterStats>(actor).unwrap();
    let attack_time_mult = world
        .get::<crate::auras::AuraStatMods>(actor)
        .map_or(1.0, |mods| {
            mods.attack_cast_time_mult * mods.attack_time_mult * mods.melee_ranged_attack_time_mult
        });
    let auto_attack_speed = attack_time_mult / (1.0 + sheet.haste_pct / 100.0);
    json!({"attribute": {"strength": sheet.primary.strength, "agility": sheet.primary.agility,
        "stamina": sheet.primary.stamina, "intellect": sheet.primary.intellect},
        "native": {"crit_pct": sheet.crit_pct, "haste_pct": sheet.haste_pct,
        "mastery_pct": sheet.mastery_pct, "mastery_points": sheet.mastery_points,
        "versatility_pct": sheet.versatility_pct, "attack_power": sheet.attack_power,
        "ranged_attack_power": sheet.ranged_attack_power, "spell_power": sheet.spell_power,
        "armor": sheet.armor, "auto_attack_speed": auto_attack_speed}, "ratings": sheet.ratings,
        "main_hand": {"min": sheet.main_hand.min, "max": sheet.main_hand.max},
        "level": world.get::<shared::components::Level>(actor).unwrap().0,
        "powers": power_snapshot(world, actor)})
}

pub(super) fn engagement(world: &World, actor: Entity, targets: &[Entity]) -> Value {
    let owner = world
        .get::<crate::followers::Follower>(actor)
        .map(|follower| follower.owner);
    let inspect = |unit| super::super::diagnostics::inspect_unit(world, None, unit);
    let target_states: Vec<_> = targets.iter().map(|&target| {
        json!({"state": inspect(target), "can_attack": crate::spell_cast::targets::relation_matches(
            world, actor, target, crate::spell_cast::targets::Relation::Enemy)})
    }).collect();
    let events = &world
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let damage = total_damage(world, targets);
    json!({"actor": inspect(actor), "owner": owner.map(inspect), "targets": target_states,
        "damage_to_targets": damage,
        "actor_is_player": world.get::<shared::components::Player>(actor).is_some(),
        "owner_is_player": owner.map(|owner| world.get::<shared::components::Player>(owner).is_some()),
        "first_events": events.iter().take(16).collect::<Vec<_>>(),
        "powers": power_snapshot(world, actor),
        "pet_powers": crate::hunter_pet::summoned_pet(world, actor).map(|pet| power_snapshot(world, pet))})
}

pub(super) fn total_damage(world: &World, targets: &[Entity]) -> i64 {
    world
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| is_target_damage(event, targets))
        .map(|event| i64::from(event.amount))
        .sum()
}

fn is_target_damage(event: &shared::protocol::CombatLogEvent, targets: &[Entity]) -> bool {
    let dummy = event
        .target
        .is_some_and(|target| targets.iter().any(|unit| unit.to_bits() == target));
    dummy && event.kind == CombatLogKind::Damage
}

pub(super) fn pet(world: &World, actor: Entity, targets: &[Entity]) -> Value {
    let Some(pet) = crate::hunter_pet::summoned_pet(world, actor) else {
        return json!({"present": false});
    };
    let events = &world
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let owned: Vec<_> = events
        .iter()
        .filter(|event| event.source == Some(pet.to_bits()))
        .collect();
    let casts: Vec<_> = owned
        .iter()
        .filter(|event| event.kind == CombatLogKind::CastSuccess)
        .filter_map(|event| event.spell_id)
        .collect();
    let damage: i64 = owned
        .iter()
        .filter(|event| is_target_damage(event, targets))
        .map(|event| i64::from(event.amount))
        .sum();
    json!({"present": true, "entity": pet.to_bits(), "casts": casts, "damage": damage,
        "entry": world.get::<shared::components::Npc>(pet).map(|npc| npc.template_id),
        "level": world.get::<shared::components::UnitLevel>(pet).map(|level| level.0),
        "action_bar": world.get::<crate::pet_control::PetCharm>(pet).map(|charm| charm.action_bar),
        "powers": power_snapshot(world, pet)})
}

pub(super) fn sample_buffs(world: &World, actor: Entity, counts: &mut BTreeMap<u32, u32>) {
    if let Some(auras) = world.get::<crate::auras::ActiveAuras>(actor) {
        for aura in auras.iter() {
            *counts.entry(aura.spell_id).or_default() += 1;
        }
    }
}

fn summarize_target_damage(events: &[CombatLogEvent], targets: &[Entity]) -> Value {
    let mut recipients: BTreeMap<_, (u64, u32)> = targets
        .iter()
        .map(|target| (target.to_bits(), (0, 0)))
        .collect();
    for event in events
        .iter()
        .filter(|event| event.kind == CombatLogKind::Damage)
    {
        let Some((damage, hits)) = event.target.and_then(|target| recipients.get_mut(&target))
        else {
            continue;
        };
        *damage += u64::try_from(event.amount).expect("nonnegative damage log");
        *hits += 1;
    }
    let rows: Vec<_> = recipients
        .into_iter()
        .map(|(target, (damage, hits))| json!({"target": target, "damage": damage, "hits": hits}))
        .collect();
    json!(rows)
}

#[derive(Default)]
struct ActionTotals {
    damage: u64,
    hits: u32,
    casts: u32,
    crits: u32,
    crit_damage: u64,
    misses: u32,
    dodges: u32,
    parries: u32,
}

fn action_report(
    actor: Option<u64>,
    spell: Option<u32>,
    totals: ActionTotals,
    total: u64,
) -> Value {
    let average = |damage: u64, hits: u32| {
        if hits > 0 {
            damage as f64 / f64::from(hits)
        } else {
            0.0
        }
    };
    json!({"actor": actor, "spell_id": spell, "damage": totals.damage, "hits": totals.hits,
        "count": totals.casts, "crit_hits": totals.crits, "crit_damage": totals.crit_damage,
        "misses": totals.misses, "dodges": totals.dodges, "parries": totals.parries,
        "noncrit_hits": totals.hits - totals.crits, "noncrit_damage": totals.damage - totals.crit_damage,
        "crit_rate": average(u64::from(totals.crits), totals.hits),
        "average_crit": average(totals.crit_damage, totals.crits),
        "average_noncrit": average(totals.damage - totals.crit_damage, totals.hits - totals.crits),
        "average_hit": average(totals.damage, totals.hits),
        "share_pct": if total > 0 { totals.damage as f64 * 100.0 / total as f64 } else { 0.0 }})
}

pub(super) fn combat(world: &World, targets: &[Entity], seconds: f64) -> Value {
    let events = &world
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let mut actions = BTreeMap::<(Option<u64>, Option<u32>), ActionTotals>::new();
    for event in events {
        let key = (event.source, event.spell_id);
        if event.kind == CombatLogKind::CastSuccess {
            actions.entry(key).or_default().casts += 1;
        }
        let dummy = event
            .target
            .is_some_and(|target| targets.iter().any(|unit| unit.to_bits() == target));
        if dummy && let CombatLogKind::Miss(kind) = event.kind {
            use shared::protocol::MissKind;
            let entry = actions.entry(key).or_default();
            match kind {
                MissKind::Miss => entry.misses += 1,
                MissKind::Dodge => entry.dodges += 1,
                MissKind::Parry => entry.parries += 1,
                _ => {}
            }
        }
        if dummy && event.kind == CombatLogKind::Damage {
            let entry = actions.entry(key).or_default();
            let amount = u64::try_from(event.amount).expect("nonnegative damage log");
            entry.damage += amount;
            entry.hits += 1;
            if event.crit {
                entry.crits += 1;
                entry.crit_damage += amount;
            }
        }
    }
    let total: u64 = actions.values().map(|entry| entry.damage).sum();
    let actions: Vec<_> = actions
        .into_iter()
        .map(|((actor, spell), totals)| action_report(actor, spell, totals, total))
        .collect();
    json!({"dps": total as f64 / seconds, "total_damage": total, "actions": actions,
        "event_count": events.len(), "damage_by_target": summarize_target_damage(events, targets),
        "energize": energize_receipts(events),
        "energize_units": "raw_logged_integer"})
}

/// Authoritative raw pools, including fractional current values. Rates describe
/// the last native tick, not integrated realized regeneration or cap losses.
fn power_snapshot(world: &World, unit: Entity) -> Value {
    use crate::spell_cast::powers::{PowerPools, power_value};
    use shared::components::PowerType;
    let Some(pools) = world.get::<PowerPools>(unit) else {
        return Value::Null;
    };
    let values: Vec<_> = pools.order.iter().map(|&power| {
        let (current, max) = power_value(world, unit, power)
            .expect("native receipt pool order requires an authoritative value");
        let regen = if power == PowerType::Mana {
            pools.mana_regen_per_sec
        } else {
            pools.get(power).expect("native receipt non-mana pool exists").regen_per_sec
        };
        json!({"power_type": power as i32, "current": current, "max": max, "regen_per_sec": regen})
    }).collect();
    json!(values)
}

/// Combat logs omit power type and truncate amounts: preserve those limits
/// instead of guessing an effect selector or equating these grants with regen.
fn energize_receipts(events: &[shared::protocol::CombatLogEvent]) -> Vec<Value> {
    let mut gains = BTreeMap::<(Option<u64>, Option<u64>, Option<u32>), (i64, i64, u64)>::new();
    for event in events
        .iter()
        .filter(|event| event.kind == CombatLogKind::Energize)
    {
        let total = gains
            .entry((event.source, event.target, event.spell_id))
            .or_default();
        total.0 += i64::from(event.amount);
        total.1 += i64::from(event.overflow);
        total.2 += 1;
    }
    gains
        .into_iter()
        .map(|((source, target, spell), (amount, overflow, count))| {
            json!({"source": source, "target": target, "spell_id": spell, "amount": amount,
            "overflow": overflow, "effective_gain": amount - overflow, "events": count})
        })
        .collect()
}

pub(super) fn gaps(profile: &super::inputs::Profile, is_player: bool) -> Vec<String> {
    let mut gaps = if is_player {
        vec![
            "NATIVE_ASSISTED_COMBAT_70448_VS_SIMC_APL".into(),
            "NATIVE_PLAYER_DECISION_INTERVAL_50MS".into(),
            "NATIVE_TARGET_ADMISSION_FOR_RANGE_FACTS".into(),
        ]
    } else {
        vec!["NATIVE_FOLLOWER_KIT_VS_SIMC_APL: rotation changes are lead-owned".into()]
    };
    gaps.extend(profile.reference_gap.iter().cloned());
    gaps.extend(
        profile
            .consumables
            .iter()
            .filter(|(kind, value)| !super::profile_effects::supported(kind, value))
            .map(|(kind, value)| format!("CONSUMABLE_UNMODELED:{kind}={value}")),
    );
    for item in &profile.equipment {
        gaps.extend(
            item.options
                .iter()
                .filter(|(key, _)| key.as_str() != "redirected_base_stats")
                .map(|(key, value)| {
                    format!("ITEM_OPTION_UNMODELED:{}:{key}={value}", item.item_id)
                }),
        );
    }
    gaps
}

#[test]
fn native_damage_recipient_accounting_preserves_retired_and_zero_output_targets() {
    let mut world = World::new();
    let actor = world.spawn_empty().id();
    let pet = world.spawn_empty().id();
    let main = world.spawn_empty().id();
    let first_add = world.spawn_empty().id();
    let second_add = world.spawn_empty().id();
    let untouched = world.spawn_empty().id();
    let unrelated = world.spawn_empty().id();
    let damage = |target: Option<Entity>, amount| CombatLogEvent {
        source: Some(actor.to_bits()),
        target: target.map(Entity::to_bits),
        spell_id: Some(116),
        school_mask: 16,
        amount,
        overflow: 0,
        absorbed: 0,
        resisted: 0,
        blocked: 0,
        crit: false,
        glancing: false,
        periodic: false,
        extra_spell_id: None,
        timestamp_unix_ms: 0,
        kind: CombatLogKind::Damage,
    };
    world.insert_resource(crate::spell_cast::test_harness::ObservedCombatEvents(vec![
        CombatLogEvent {
            crit: true,
            ..damage(Some(main), 100)
        },
        CombatLogEvent {
            source: Some(pet.to_bits()),
            periodic: true,
            ..damage(Some(first_add), 200)
        },
        damage(Some(second_add), 300),
        damage(Some(second_add), 0),
        CombatLogEvent {
            kind: CombatLogKind::Heal,
            ..damage(Some(second_add), 900)
        },
        damage(Some(unrelated), 999),
        damage(None, 999),
        CombatLogEvent {
            kind: CombatLogKind::CastSuccess,
            ..damage(None, 0)
        },
    ]));
    world.despawn(first_add);

    let report = combat(&world, &[main, first_add, second_add, untouched], 300.0);
    assert_eq!(report["total_damage"], 600);
    assert_eq!(report["dps"], 2.0);
    let recipients: BTreeMap<_, _> = report["damage_by_target"]
        .as_array()
        .expect("native receipt must retain damage recipients")
        .iter()
        .map(|row| {
            (
                row["target"].as_u64().unwrap(),
                (
                    row["damage"].as_u64().unwrap(),
                    row["hits"].as_u64().unwrap(),
                ),
            )
        })
        .collect();
    assert_eq!(
        recipients,
        BTreeMap::from([
            (main.to_bits(), (100, 1)),
            (first_add.to_bits(), (200, 1)),
            (second_add.to_bits(), (300, 2)),
            (untouched.to_bits(), (0, 0)),
        ])
    );
    let actions = report["actions"].as_array().unwrap();
    let owner_action = actions
        .iter()
        .find(|row| row["actor"] == actor.to_bits())
        .unwrap();
    assert_eq!(owner_action["damage"], 400);
    assert_eq!(owner_action["hits"], 3);
    assert_eq!(owner_action["count"], 1);
    assert_eq!(owner_action["crit_hits"], 1);
    assert_eq!(owner_action["crit_damage"], 100);
    assert_eq!(owner_action["noncrit_hits"], 2);
    assert_eq!(owner_action["noncrit_damage"], 300);
    assert_eq!(owner_action["crit_rate"], 1.0 / 3.0);
    assert_eq!(owner_action["average_crit"], 100.0);
    assert_eq!(owner_action["average_noncrit"], 150.0);
    let pet_action = actions
        .iter()
        .find(|row| row["actor"] == pet.to_bits())
        .unwrap();
    assert_eq!(pet_action["damage"], 200);
    assert_eq!(pet_action["hits"], 1);
    assert_eq!(pet_action["count"], 0);
}

#[test]
fn native_action_report_counts_miss_dodge_parry_and_keeps_miss_only_abilities() {
    use shared::protocol::MissKind;
    let mut world = World::new();
    let actor = world.spawn_empty().id();
    let target = world.spawn_empty().id();
    let foreign = world.spawn_empty().id();
    let event = |spell, kind, recipient| CombatLogEvent {
        source: Some(actor.to_bits()),
        target: Some(recipient),
        spell_id: Some(spell),
        school_mask: 2,
        amount: 0,
        overflow: 0,
        absorbed: 0,
        resisted: 0,
        blocked: 0,
        crit: false,
        glancing: false,
        periodic: false,
        extra_spell_id: None,
        timestamp_unix_ms: 0,
        kind,
    };
    let at = target.to_bits();
    world.insert_resource(crate::spell_cast::test_harness::ObservedCombatEvents(vec![
        CombatLogEvent {
            amount: 120,
            ..event(383328, CombatLogKind::Damage, at)
        },
        event(383328, CombatLogKind::CastSuccess, at),
        event(383328, CombatLogKind::Miss(MissKind::Miss), at),
        event(383328, CombatLogKind::Miss(MissKind::Miss), at),
        event(383328, CombatLogKind::Miss(MissKind::Dodge), at),
        event(383328, CombatLogKind::Miss(MissKind::Parry), at),
        event(
            383328,
            CombatLogKind::Miss(MissKind::Dodge),
            foreign.to_bits(),
        ),
        event(408385, CombatLogKind::Miss(MissKind::Dodge), at),
    ]));
    let report = combat(&world, &[target], 10.0);
    assert_eq!(report["total_damage"], 120);
    let actions = report["actions"].as_array().unwrap();
    let verdict = actions.iter().find(|a| a["spell_id"] == 383328).unwrap();
    assert_eq!(verdict["hits"], 1);
    assert_eq!(verdict["count"], 1);
    assert_eq!(verdict["misses"], 2);
    assert_eq!(verdict["dodges"], 1);
    assert_eq!(verdict["parries"], 1);
    let strike = actions
        .iter()
        .find(|a| a["spell_id"] == 408385)
        .expect("a miss-only background ability must not disappear");
    assert_eq!(strike["hits"], 0);
    assert_eq!(strike["damage"], 0);
    assert_eq!(strike["misses"], 0);
    assert_eq!(strike["dodges"], 1);
    assert_eq!(strike["parries"], 0);
}
