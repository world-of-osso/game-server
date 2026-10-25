//! BM_SNAKESKIN_QUIVER_SIMC_76F8118: sc_hunter.cpp3988-4007/5041-5047.
//! Native468695 supplies the chance; successful Auto Shot execute launches free Cobra.
use bevy::prelude::*;

const SNAKESKIN_QUIVER: u32 = 468695;
const COBRA_SHOT: u32 = 193455;

pub(in crate::spell_cast) fn roll_snakeskin_cobra(
    world: &mut World,
    caster: Entity,
    target: Entity,
) {
    if !crate::auras::has_aura(world, caster, SNAKESKIN_QUIVER) {
        return;
    }
    let chance = super::script_effect_value(world, caster, SNAKESKIN_QUIVER, 0);
    if crate::random::random::<f32>() * 100.0 >= chance {
        return;
    }
    if !crate::spell_cast::try_cast_triggered(world, caster, COBRA_SHOT, Some(target), 1) {
        warn!(?caster, ?target, "Snakeskin free Cobra Shot refused");
    }
}
