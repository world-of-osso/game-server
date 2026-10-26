//! Native charge damage is one launch-snapshotted spell modifier.
use bevy::prelude::*;

use crate::auras::{ActiveAuras, AuraEffectState, AuraInstance};
use crate::spell_info::SpellStore;

pub(crate) fn modifier_amount(
    world: &World,
    caster: Entity,
    aura: &AuraInstance,
    effect: &AuraEffectState,
) -> Option<f32> {
    if aura.spell_id != 36032 || effect.index > 1 {
        return None;
    }
    let store = world.resource::<SpellStore>();
    let charge = store.get(aura.spell_id).expect("native Arcane Charge");
    let base = charge
        .effects
        .iter()
        .find(|row| row.index == effect.index)
        .expect("native charge damage effect")
        .base_points;
    let mastery = world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| auras.iter().find(|row| row.spell_id == 190740))
        .and_then(|aura| {
            aura.effects
                .iter()
                .find(|row| row.index == effect.index + 1)
        })
        .map_or(0.0, |row| row.amount);
    // SimC76f8118 sc_mage.cpp2076–2083: Savant modifies the per-charge
    // term; Prodigious Savant multiplies only mastery, not the base charge bonus.
    let prodigious = world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| auras.iter().find(|row| row.spell_id == 384612))
        .and_then(|aura| aura.effects.iter().find(|row| row.index == effect.index))
        .map_or(0.0, |row| row.amount * aura.stack_multiplier());
    Some((base + mastery * (1.0 + prodigious / 100.0)) * aura.stack_multiplier())
}
