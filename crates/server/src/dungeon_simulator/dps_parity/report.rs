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
        "level": world.get::<shared::components::Level>(actor).unwrap().0})
}

pub(super) fn engagement(world: &World, actor: Entity, targets: &[Entity]) -> Value {
    let owner = world
        .get::<crate::followers::Follower>(actor)
        .unwrap()
        .owner;
    let inspect = |unit| super::super::diagnostics::inspect_unit(world, None, unit);
    let target_states: Vec<_> = targets.iter().map(|&target| {
        json!({"state": inspect(target), "can_attack": crate::spell_cast::targets::relation_matches(
            world, actor, target, crate::spell_cast::targets::Relation::Enemy)})
    }).collect();
    let events = &world
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    json!({"actor": inspect(actor), "owner": inspect(owner), "targets": target_states,
        "first_events": events.iter().take(16).collect::<Vec<_>>()})
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
        "event_count": events.len()})
}

pub(super) fn gaps(profile: &super::inputs::Profile) -> Vec<String> {
    let mut gaps = vec![
        format!("FOLLOWER_RACE0_VS_REFERENCE_{}", profile.race),
        "NATIVE_FOLLOWER_KIT_VS_SIMC_APL: rotation changes are lead-owned".into(),
    ];
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
