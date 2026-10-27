//! FoF retains full damage only for its first native TargetInfo.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::effects::EffectCtx;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::scripts::script_effect_value;

const FISTS_OF_FURY: u32 = 113656;
const DAMAGE: u32 = 117418;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::TargetContextDamageMultiplier(
    secondary_multiplier,
)];

fn secondary_multiplier(world: &World, ctx: &EffectCtx<'_>, target: Entity, _: bool) -> f32 {
    if ctx.spell.id != DAMAGE {
        return 1.0;
    }
    // SimC76f8118 monk1690–1717: chain_target0 is full; all other impacts
    // use native FoF effect6, including the MID2 tick_t child (1809–1813).
    let primary = combat::spell_cast::resolving_targets(world, ctx)
        .and_then(|targets| targets.first())
        .expect("native FoF damage retains its first TargetInfo");
    if *primary == target {
        return 1.0;
    }
    script_effect_value(world, ctx.caster, FISTS_OF_FURY, 5) / 100.0
}
