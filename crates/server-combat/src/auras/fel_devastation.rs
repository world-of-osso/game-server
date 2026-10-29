//! Native70448 scheduled Fel Devastation healing; TC a352b1fa spell_dh1291–1310.
//! SimC76f8118 DH4204–4212 agrees on heal-before-damage periodic order.
//! The separate cast heal is registered in vengeance_fel_devastation.
use bevy::prelude::*;

use super::AuraInstance;

pub(super) const FEL_DEVASTATION: u32 = 212_084;
const FEL_DEVASTATION_HEAL: u32 = 212_106;
const FEL_DEVASTATION_RANK_TWO: u32 = 320_639;
const METEORIC_RISE: u32 = 1_253_377;

/// SimC76f8118 sc_demon_hunter.cpp4211–4220 distributes the talent's
/// fragments over the scheduled ticks, after each tick's damage.
pub(super) fn generate_meteoric_rise_fragment(
    world: &mut World,
    unit: Entity,
    instance: u32,
    ordinal: u64,
) {
    if ordinal == 0 || !super::has_aura(world, unit, METEORIC_RISE) {
        return;
    }
    let Some(aura) = world
        .get::<super::ActiveAuras>(unit)
        .and_then(|auras| auras.get(instance))
    else {
        return;
    };
    let total_ticks = ordinal
        + u64::from(
            super::periodic::count_remaining_ticks(aura)
                .expect("Fel Devastation has a finite tick schedule"),
        );
    let count =
        crate::spell_cast::scripts::script_effect_value(world, unit, METEORIC_RISE, 3) as u64;
    if ordinal * count / total_ticks > (ordinal - 1) * count / total_ticks {
        crate::soul_fragments::spawn(
            world,
            unit,
            crate::soul_fragments::FragmentKind::Lesser,
            false,
        )
        .expect("native Meteoric Rise fragment");
    }
}

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
