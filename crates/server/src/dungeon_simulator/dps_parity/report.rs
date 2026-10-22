//! Actual combat log accounting; no inferred hits or fabricated stat contributions.
use bevy::prelude::*;
use serde_json::{Value, json};
use shared::protocol::CombatLogKind;
use std::collections::BTreeMap;

pub(super) fn stats(world: &World, actor: Entity) -> Value {
    let sheet = world.get::<crate::stats::CharacterStats>(actor).unwrap();
    json!({"attribute": {"strength": sheet.primary.strength, "agility": sheet.primary.agility,
        "stamina": sheet.primary.stamina, "intellect": sheet.primary.intellect},
        "native": {"crit_pct": sheet.crit_pct, "haste_pct": sheet.haste_pct,
        "mastery_pct": sheet.mastery_pct, "mastery_points": sheet.mastery_points,
        "versatility_pct": sheet.versatility_pct, "attack_power": sheet.attack_power,
        "ranged_attack_power": sheet.ranged_attack_power, "spell_power": sheet.spell_power,
        "armor": sheet.armor}, "ratings": sheet.ratings,
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
    let damage: i64 = events
        .iter()
        .filter(|event| is_target_damage(event, targets))
        .map(|event| i64::from(event.amount))
        .sum();
    json!({"actor": inspect(actor), "owner": owner.map(inspect), "targets": target_states,
        "damage_to_targets": damage,
        "actor_is_player": world.get::<shared::components::Player>(actor).is_some(),
        "owner_is_player": owner.map(|owner| world.get::<shared::components::Player>(owner).is_some()),
        "first_events": events.iter().take(16).collect::<Vec<_>>(),
        "powers": power_snapshot(world, actor),
        "pet_powers": crate::hunter_pet::summoned_pet(world, actor).map(|pet| power_snapshot(world, pet))})
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

pub(super) fn combat(world: &World, targets: &[Entity], seconds: f64) -> Value {
    let events = &world
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let mut actions = BTreeMap::<(Option<u64>, Option<u32>), (u64, u32, u32)>::new();
    for event in events {
        let key = (event.source, event.spell_id);
        if event.kind == CombatLogKind::CastSuccess {
            actions.entry(key).or_default().2 += 1;
        }
        let dummy = event
            .target
            .is_some_and(|target| targets.iter().any(|unit| unit.to_bits() == target));
        if dummy && event.kind == CombatLogKind::Damage {
            let entry = actions.entry(key).or_default();
            entry.0 += u64::try_from(event.amount).expect("nonnegative damage log");
            entry.1 += 1;
        }
    }
    let total: u64 = actions.values().map(|entry| entry.0).sum();
    let actions: Vec<_> = actions.into_iter().map(|((actor, spell), (damage, hits, casts))| {
        json!({"actor": actor, "spell_id": spell, "damage": damage, "hits": hits, "count": casts,
            "average_hit": if hits > 0 { damage as f64 / f64::from(hits) } else { 0.0 },
            "share_pct": if total > 0 { damage as f64 * 100.0 / total as f64 } else { 0.0 }})
    }).collect();
    json!({"dps": total as f64 / seconds, "total_damage": total, "actions": actions,
        "event_count": events.len(), "energize": energize_receipts(events),
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
        vec![
            format!("FOLLOWER_RACE0_VS_REFERENCE_{}", profile.race),
            "NATIVE_FOLLOWER_KIT_VS_SIMC_APL: rotation changes are lead-owned".into(),
        ]
    };
    gaps.extend(profile.reference_gap.iter().cloned());
    if profile.omnium_talents.is_some() {
        gaps.push("SIMC_OMNIUM_TALENTS_UNMODELED".into());
    }
    gaps.extend(
        profile
            .consumables
            .iter()
            .map(|(kind, value)| format!("CONSUMABLE_UNMODELED:{kind}={value}")),
    );
    for item in &profile.equipment {
        gaps.extend(
            item.options.iter().map(|(key, value)| {
                format!("ITEM_OPTION_UNMODELED:{}:{key}={value}", item.item_id)
            }),
        );
    }
    gaps
}
