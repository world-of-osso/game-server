//! TC a352b1fa SpellHistory.cpp432-455/608-628 and SpellAuraEffects.cpp6258:
//! recovery-rate effects scale positive cooldowns and remaining active timers.
use bevy::prelude::*;
use std::collections::BTreeSet;

use super::{SpellHistory, history_now, send_to_owner};
use crate::auras::{ActiveAuras, AuraEffectState, AuraInstance, handlers, spell_mods};
use crate::spell_info::{SpellInfo, SpellStore};
use shared::protocol::SpellCooldownUpdate;

fn is_recovery_effect(effect: &AuraEffectState) -> bool {
    matches!(
        effect.aura_type,
        handlers::MOD_RECOVERY_RATE | handlers::MOD_RECOVERY_RATE_BY_SPELL_LABEL
    )
}

fn affects_spell(aura: &AuraInstance, effect: &AuraEffectState, spell: &SpellInfo) -> bool {
    match effect.aura_type {
        handlers::MOD_RECOVERY_RATE => spell_mods::class_mask_affects(aura, effect, spell),
        handlers::MOD_RECOVERY_RATE_BY_SPELL_LABEL => {
            let [primary, secondary] = effect.misc_values;
            spell.labels.contains(&(primary as u32))
                || (secondary != 0 && spell.labels.contains(&(secondary as u32)))
        }
        _ => false,
    }
}

fn aura_multiplier(aura: &AuraInstance, spell: &SpellInfo) -> f64 {
    aura.effects
        .iter()
        .filter(|effect| affects_spell(aura, effect, spell))
        .map(|effect| {
            let amount = effect.amount * aura.stack_multiplier();
            100.0 / (100.0 + f64::from(amount.max(-99.0)))
        })
        .product()
}

pub(crate) fn multiplier(world: &World, unit: Entity, spell: &SpellInfo) -> f64 {
    world.get::<ActiveAuras>(unit).map_or(1.0, |auras| {
        auras
            .iter()
            .map(|aura| aura_multiplier(aura, spell))
            .product()
    })
}

pub(crate) fn capture_previous_rate_aura(
    world: &World,
    unit: Entity,
    incoming: &AuraInstance,
) -> Option<AuraInstance> {
    world
        .get::<ActiveAuras>(unit)?
        .iter()
        .find(|old| {
            old.spell_id == incoming.spell_id
                && old.caster == incoming.caster
                && old.source == incoming.source
                && old.effects.iter().any(is_recovery_effect)
        })
        .cloned()
}

fn rates_unchanged(previous: &AuraInstance, current: &AuraInstance) -> bool {
    let rates = |aura: &AuraInstance| {
        aura.effects
            .iter()
            .filter(|effect| is_recovery_effect(effect))
            .map(|effect| {
                (
                    effect.aura_type,
                    effect.class_mask,
                    effect.misc_values,
                    effect.amount * aura.stack_multiplier(),
                )
            })
            .collect::<Vec<_>>()
    };
    previous.class_set == current.class_set && rates(previous) == rates(current)
}

pub(crate) fn on_refreshed(
    world: &mut World,
    unit: Entity,
    previous: Option<&AuraInstance>,
    current: &AuraInstance,
) {
    if let Some(previous) = previous {
        if rates_unchanged(previous, current) {
            return;
        }
        on_removed(world, unit, previous);
    }
    on_applied(world, unit, current);
}

pub(crate) fn on_applied(world: &mut World, unit: Entity, aura: &AuraInstance) {
    retime_running_cooldowns(world, unit, aura, true);
}

pub(crate) fn on_removed(world: &mut World, unit: Entity, aura: &AuraInstance) {
    retime_running_cooldowns(world, unit, aura, false);
}

fn calculate_deadline_or_warn(
    store: &SpellStore,
    aura: &AuraInstance,
    spell_id: u32,
    deadline: f64,
    current_time: f64,
    apply: bool,
) -> Option<f64> {
    if deadline <= current_time {
        return None;
    }
    let Some(spell) = store.get(spell_id) else {
        warn!(
            "Recovery-rate aura{}: active cooldown spell{spell_id} has no metadata",
            aura.spell_id
        );
        return None;
    };
    let factor = aura_multiplier(aura, spell);
    if factor == 1.0 {
        return None;
    }
    let factor = if apply { factor } else { factor.recip() };
    // Native duration_cast truncates to integer milliseconds.
    let remaining_ms = ((deadline - current_time) * 1000.0 * factor).trunc();
    Some(current_time + remaining_ms / 1000.0)
}

fn retime_running_cooldowns(world: &mut World, unit: Entity, aura: &AuraInstance, apply: bool) {
    if !aura.effects.iter().any(is_recovery_effect) {
        return;
    }
    let Some(snapshot) = world.get::<SpellHistory>(unit).cloned() else {
        return;
    };
    let current_time = history_now(world);
    let store = world.resource::<SpellStore>();
    let spell_updates = collect_spell_updates(store, &snapshot, aura, current_time, apply);
    let category_updates = collect_category_updates(store, &snapshot, aura, current_time, apply);
    let affected: BTreeSet<_> = spell_updates
        .iter()
        .map(|(id, _)| *id)
        .chain(category_updates.iter().map(|(_, id, _)| *id))
        .collect();
    write_deadlines(world, unit, spell_updates, category_updates);
    publish_cooldowns(world, unit, &affected, current_time);
}

fn collect_spell_updates(
    store: &SpellStore,
    history: &SpellHistory,
    aura: &AuraInstance,
    current_time: f64,
    apply: bool,
) -> Vec<(u32, f64)> {
    history
        .spell_ready_at
        .iter()
        .filter_map(|(&id, &deadline)| {
            calculate_deadline_or_warn(store, aura, id, deadline, current_time, apply)
                .map(|deadline| (id, deadline))
        })
        .collect()
}

fn collect_category_updates(
    store: &SpellStore,
    history: &SpellHistory,
    aura: &AuraInstance,
    current_time: f64,
    apply: bool,
) -> Vec<(u32, u32, f64)> {
    history
        .category_spell_ids
        .iter()
        .filter_map(|(&category, &id)| {
            let deadline = history.category_ready_at.get(&category)?;
            calculate_deadline_or_warn(store, aura, id, *deadline, current_time, apply)
                .map(|deadline| (category, id, deadline))
        })
        .collect()
}

fn write_deadlines(
    world: &mut World,
    unit: Entity,
    spells: Vec<(u32, f64)>,
    categories: Vec<(u32, u32, f64)>,
) {
    let mut history = world
        .get_mut::<SpellHistory>(unit)
        .expect("active cooldown owner");
    for (id, deadline) in spells {
        history.spell_ready_at.insert(id, deadline);
    }
    for (category, _, deadline) in categories {
        history.category_ready_at.insert(category, deadline);
    }
}

fn build_cooldown_update(
    history: &SpellHistory,
    id: u32,
    current_time: f64,
) -> SpellCooldownUpdate {
    let category = history
        .category_spell_ids
        .iter()
        .find_map(|(&category, &owner)| (owner == id).then_some(category))
        .unwrap_or(0);
    let category_deadline = history
        .category_ready_at
        .get(&category)
        .copied()
        .unwrap_or(current_time);
    let spell_deadline = history
        .spell_ready_at
        .get(&id)
        .copied()
        .unwrap_or(current_time);
    let remaining_ms =
        ((spell_deadline.max(category_deadline) - current_time).max(0.0) * 1000.0).round() as u32;
    SpellCooldownUpdate {
        spell_id: id,
        category,
        duration_ms: remaining_ms,
        remaining_ms,
        is_gcd: false,
    }
}

fn publish_cooldowns(world: &mut World, unit: Entity, affected: &BTreeSet<u32>, current_time: f64) {
    let history = world
        .get::<SpellHistory>(unit)
        .expect("updated cooldown owner");
    let updates: Vec<_> = affected
        .iter()
        .map(|&id| build_cooldown_update(history, id, current_time))
        .collect();
    for update in updates {
        send_to_owner(world, unit, update);
    }
}
