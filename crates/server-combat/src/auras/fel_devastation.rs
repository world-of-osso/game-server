//! Native70448 scheduled Fel Devastation healing; TC a352b1fa spell_dh1291–1310.
//! SimC76f8118 DH4204–4212 agrees on heal-before-damage periodic order.
//! FEL_DEVASTATION_UPFRONT_HEAL_SOURCE_CONFLICT: SimC also heals on execute;
//! that difference and full aura-origin/proc/trigger-flag parity remain deferred.
use bevy::prelude::*;

use super::AuraInstance;

pub(super) const FEL_DEVASTATION: u32 = 212_084;
const FEL_DEVASTATION_HEAL: u32 = 212_106;
const FEL_DEVASTATION_RANK_TWO: u32 = 320_639;

pub(super) fn cast_periodic_heal(
    world: &mut World,
    _unit: Entity,
    aura: &AuraInstance,
    effect_index: u8,
) {
    if effect_index != 0 || !super::has_aura(world, aura.caster, FEL_DEVASTATION_RANK_TWO) {
        return;
    }
    crate::spell_cast::cast_triggered(
        world,
        aura.caster,
        FEL_DEVASTATION_HEAL,
        Some(aura.caster),
        1,
    );
}
