//! BM_SNAKESKIN_QUIVER_SIMC_76F8118: sc_hunter.cpp3988-4007/5041-5047.
//! Native468695 supplies the chance; successful Auto Shot execute launches free Cobra.
use bevy::prelude::*;
use super::combat;
use combat::spell_cast::script_registry::SpellScriptHook;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AutoRepeat(roll_snakeskin_cobra)];

const SNAKESKIN_QUIVER: u32 = 468695;
const COBRA_SHOT: u32 = 193455;

fn roll_snakeskin_cobra(
    world: &mut World,
    caster: Entity,
    target: Entity,
) {
    if !combat::auras::has_aura(world, caster, SNAKESKIN_QUIVER) {
        return;
    }
    let chance = combat::spell_cast::scripts::script_effect_value(world, caster, SNAKESKIN_QUIVER, 0);
    if combat::random::random::<f32>() * 100.0 >= chance {
        return;
    }
    if !combat::spell_cast::try_cast_triggered(world, caster, COBRA_SHOT, Some(target), 1) {
        warn!(?caster, ?target, "Snakeskin free Cobra Shot refused");
    }
}
