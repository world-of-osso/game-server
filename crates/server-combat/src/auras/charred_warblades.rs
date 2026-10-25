//! TC a352b1fa spell_dh.cpp480-519: collect eligible Fire damage per aura
//! instance; effect0's periodic tick casts the accumulated integer heal once.
use std::collections::HashMap;

use bevy::prelude::*;

use super::procs::AuraEvent;
use super::{AuraInstance, RemoveReason};
use crate::spell_info::SpellStore;

pub(super) const CHARRED_WARBLADES: u32 = 213_010;
const CHARRED_HEAL: u32 = 213_011;
const FIRE_SCHOOL: u32 = 4;

#[derive(Component, Default)]
struct PendingHeals(HashMap<u32, u32>);

pub(super) fn check_fire_damage(world: &World, _aura: &AuraInstance, event: &AuraEvent) -> bool {
    event.spell_id.is_some_and(|spell| {
        world
            .resource::<SpellStore>()
            .get(spell)
            .is_some_and(|spell| spell.school_mask() & FIRE_SCHOOL != 0)
    })
}

pub(super) fn accumulate_heal(
    world: &mut World,
    unit: Entity,
    aura: &AuraInstance,
    event: &AuraEvent,
) {
    let Some(effect) = aura.effects.iter().find(|effect| effect.index == 0) else {
        return;
    };
    // TC DamageInfo::GetDamage and the uint32 accumulator truncate each event,
    // not the sum of fractional contributions at the later periodic tick.
    let damage = event.damage as u32;
    let amount = (damage as f64 * f64::from(effect.amount) / 100.0) as u32;
    let mut entity = world.entity_mut(unit);
    let mut pending = entity.entry::<PendingHeals>().or_default();
    *pending.get_mut().0.entry(aura.instance_id).or_default() += amount;
}

pub(super) fn heal_on_tick(world: &mut World, unit: Entity, aura: &AuraInstance, index: u8) {
    if index != 0 {
        return;
    }
    let amount = world
        .get_mut::<PendingHeals>(unit)
        .and_then(|mut pending| pending.0.remove(&aura.instance_id))
        .unwrap_or(0);
    if amount == 0 {
        return;
    }
    crate::spell_cast::cast_triggered_with_effect0_amount(
        world,
        unit,
        CHARRED_HEAL,
        Some(unit),
        1,
        amount as f32,
    );
}

pub(super) fn discard_heal(
    world: &mut World,
    unit: Entity,
    aura: &AuraInstance,
    _reason: RemoveReason,
) {
    if let Some(mut pending) = world.get_mut::<PendingHeals>(unit) {
        pending.0.remove(&aura.instance_id);
    }
}
