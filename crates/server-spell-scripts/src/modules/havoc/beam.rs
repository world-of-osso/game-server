//! Native Eye Beam/Gaze channel children and the learned Eye Beam critical rank.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{self, effects::EffectCtx, script_registry::ContextHitHook};
use shared::casting::{CastState, CastType};

const EYE_BEAM: u32 = 198013;
const GAZE: u32 = 452497;
const EYE_BEAM_DAMAGE: u32 = 198030;
const EYE_BEAM_RANK: u32 = 320415;

pub(super) fn periodic_trigger(spell: u32, index: u8) -> Option<ContextHitHook> {
    (matches!(spell, EYE_BEAM | GAZE) && index == 0).then_some(cast_tick)
}

///198013's local variable316 names198030;320415 explicitly guarantees crit.
///No rank means ordinary generic crit, not a forced noncritical hit.
fn cast_tick(world: &mut World, ctx: &EffectCtx<'_>, _unit: Entity) {
    let caster = ctx.caster;
    let parent = ctx.spell.id;
    let channel = world
        .get::<CastState>(caster)
        .is_some_and(|cast| cast.cast_type == CastType::Channel && cast.spell_id == parent);
    if !channel {
        return;
    }
    // SimC76f8118 sc_demon_hunter.cpp3925–4050: both channels use native
    // cone children, not a hand-built single-target plan or the self cast unit.
    let child = if combat::auras::has_aura(world, caster, 1271144) {
        1287949
    } else {
        EYE_BEAM_DAMAGE
    };
    spell_cast::cast_triggered(world, caster, child, None, 1);
}

pub(super) fn crit_chance(world: &World, caster: Entity, _: Entity, spell: u32) -> Option<f32> {
    if spell != EYE_BEAM_DAMAGE {
        return None;
    }
    combat::auras::has_aura(world, caster, EYE_BEAM_RANK).then_some(100.0)
}
