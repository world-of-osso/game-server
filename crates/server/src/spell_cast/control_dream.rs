//! Native434249: unused ready time, capped by s1 seconds, reduces the next cooldown.
//! IDs follow the named abilities in local Mainline12.1.0.69933 Spell/SpellName.
use std::collections::{HashMap, HashSet};

use bevy::prelude::*;

use crate::auras::{ActiveAuras, AuraInstance, RemoveReason};
use crate::spell_info::SpellInfo;

pub(crate) const CONTROL_OF_THE_DREAM: u32 = 434_249;

#[derive(Component, Default)]
struct DreamReadiness(HashMap<u32, f64>);

pub(crate) fn start_ready_credit(world: &mut World, unit: Entity, aura: &AuraInstance) {
    if aura.caster != unit || world.get::<DreamReadiness>(unit).is_some() {
        return;
    }
    let spells = world
        .get::<crate::class_progression::KnownSpells>(unit)
        .map_or_else(Vec::new, |known| known.iter().collect());
    world.entity_mut(unit).insert(DreamReadiness::default());
    sync_known_ready_credit(world, unit, &spells);
}

pub(crate) fn stop_ready_credit(
    world: &mut World,
    unit: Entity,
    aura: &AuraInstance,
    _reason: RemoveReason,
) {
    if aura.caster == unit {
        world.entity_mut(unit).remove::<DreamReadiness>();
    }
}

fn ability_key(spell_id: u32) -> Option<u32> {
    match spell_id {
        194_223 | 383_410 | 395_022 => Some(194_223),
        102_560 | 390_414 | 394_013 => Some(102_560),
        132_158 | 33_891 | 205_636 | 391_528 => Some(spell_id),
        _ => None,
    }
}

/// A newly acquired ability was not available during earlier talent time.
pub(crate) fn sync_known_ready_credit(world: &mut World, unit: Entity, spells: &[u32]) {
    let now = super::history_now(world);
    let Some(mut readiness) = world.get_mut::<DreamReadiness>(unit) else {
        return;
    };
    let known: HashSet<_> = spells.iter().filter_map(|&id| ability_key(id)).collect();
    readiness.0.retain(|id, _| known.contains(id));
    for id in known {
        readiness.0.entry(id).or_insert(now);
    }
}

fn native_cap_seconds(world: &World, unit: Entity) -> Option<f64> {
    let aura = world
        .get::<ActiveAuras>(unit)?
        .iter()
        .find(|aura| aura.spell_id == CONTROL_OF_THE_DREAM && aura.caster == unit)?;
    let amount = aura.effects.iter().find(|effect| effect.index == 0)?.amount;
    Some(f64::from(amount))
}

fn fully_ready_at(history: &super::SpellHistory, spell: &SpellInfo, now: f64) -> Option<f64> {
    let category = spell.categories.as_ref().map_or(0, |data| data.category);
    let spell_ready = history
        .spell_ready_at
        .get(&spell.id)
        .copied()
        .unwrap_or(0.0);
    let category_ready = history
        .category_ready_at
        .get(&category)
        .copied()
        .unwrap_or(0.0);
    let mut ready = spell_ready.max(category_ready);
    if let Some(charges) = spell
        .categories
        .as_ref()
        .and_then(|data| history.charges.get(&data.charge_category))
    {
        let mut charges = charges.clone();
        charges.refresh(now);
        if charges.used() > 0 {
            return None;
        }
        ready = ready.max(charges.fully_ready_at);
    }
    (ready <= now).then_some(ready)
}

pub(super) fn ready_credit_ms(world: &World, unit: Entity, spell: &SpellInfo, now: f64) -> u32 {
    let Some(key) = ability_key(spell.id) else {
        return 0;
    };
    let learned_at = world
        .get::<DreamReadiness>(unit)
        .and_then(|state| state.0.get(&key))
        .copied();
    let Some(learned_at) = learned_at else {
        return 0;
    };
    let Some(cap) = native_cap_seconds(world, unit) else {
        return 0;
    };
    let ready = world
        .get::<super::SpellHistory>(unit)
        .map_or(Some(learned_at), |history| {
            fully_ready_at(history, spell, now)
        });
    let Some(ready) = ready else {
        return 0;
    };
    let elapsed = (now - ready.max(learned_at)).max(0.0).min(cap);
    (elapsed * 1_000.0).round() as u32
}
