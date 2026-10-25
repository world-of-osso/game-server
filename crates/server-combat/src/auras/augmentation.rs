//! Spell-specific support-stat bindings, not a generic MOD_SUPPORT_STAT540
//! implementation. Active70448 amounts; SimC76f8118 sc_evoker.cpp3383/7611/8538/8968.
use bevy::prelude::*;

use super::{ActiveAuras, AuraInstance};
use crate::stats::CharacterStats;

pub(crate) const PRESCIENCE: u32 = 409_311;
pub(crate) const PRESCIENCE_BUFF: u32 = 410_089;
pub(crate) const SHIFTING_SANDS: u32 = 413_984;

/// Only the explicit-friendly effect1 is implemented; automatic prioritization
/// and effect0 remain separate. The child retains its native effects and gaps.
pub(crate) fn grant_prescience(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    crate::spell_cast::cast_triggered(world, caster, PRESCIENCE_BUFF, Some(target), depth + 1);
}

pub(super) fn consumes_field(spell: u32, index: u8) -> bool {
    matches!(
        (spell, index),
        (PRESCIENCE_BUFF, 0) | (SHIFTING_SANDS, 0 | 1)
    )
}

fn caster_mastery(world: &World, caster: Entity) -> f32 {
    world
        .get::<CharacterStats>(caster)
        .expect("Shifting Sands caster requires computed mastery")
        .mastery_pct
}

/// SimC shifting_sands_t::impact supplies the caster's current mastery value,
/// independently of the recipient's stats and the zero native support amount.
pub(super) fn snapshot_amount(
    world: &World,
    caster: Entity,
    spell: u32,
    index: u8,
    native: f32,
) -> f32 {
    if (spell, index) == (SHIFTING_SANDS, 0) {
        caster_mastery(world, caster)
    } else {
        native
    }
}

/// The source's Shifting Sands tick updates only its own support contribution;
/// neither expiry, stacks nor the other caster's instances are refreshed.
pub(super) fn refresh_sands(world: &mut World, unit: Entity, aura: &AuraInstance, index: u8) {
    if index != 1 {
        return;
    }
    let amount = caster_mastery(world, aura.caster);
    let mut auras = world
        .get_mut::<ActiveAuras>(unit)
        .expect("ticking aura owner");
    let effect = auras
        .get_mut(aura.instance_id)
        .expect("ticking Shifting Sands instance")
        .effects
        .iter_mut()
        .find(|effect| effect.index == 0)
        .expect("Shifting Sands support effect");
    if effect.amount != amount {
        effect.amount = amount;
        super::auras_changed(world, unit);
    }
}
