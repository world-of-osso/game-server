//! Selected Death's Embrace scales source-qualified damage with live recipient health.
use super::combat;
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::spell_cast::{effects::EffectCtx, script_registry::SpellScriptHook, scripts::script_effect_value};
use shared::components::{ActiveSpec, Health};

const EMBRACE: u32 = 234876;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::TargetContextDamageMultiplier(damage_multiplier),
];

// SimC76f8118 actions767/795/809/832/1081/1198/1417/1504/1721/1791/
// 2012/2105/2285: pinned bugs=1 excludes direct Corruption/Wither and Volley.
// Malefic Grasp itself has no affected flag; its extra DoT payloads do.
fn affected(spell: u32, periodic: bool) -> bool {
    match spell {
        146739 | 445474 => periodic,
        686 | 980 | 1259790 | 27243 | 27285 | 1261166 | 1261176 | 1261158 | 1279686 | 198590 => true,
        _ => false,
    }
}

fn selected(world: &World, caster: Entity) -> bool {
    world.get::<ActiveSpec>(caster).is_some_and(|spec| spec.0 == 265)
        && world.get::<ActiveAuras>(caster).is_some_and(|auras| {
            auras.iter().any(|aura| aura.caster == caster && aura.spell_id == EMBRACE)
        })
}

// SimC76f8118 actions529–535: evaluate the current recipient, not a cast
// snapshot or the caster's selected target. Native effects supply40% and35%.
fn damage_multiplier(world: &World, ctx: &EffectCtx<'_>, target: Entity, periodic: bool) -> f32 {
    if !affected(ctx.spell.id, periodic) || !selected(world, ctx.caster) {
        return 1.0;
    }
    let Some(health) = world.get::<Health>(target) else { return 1.0; };
    let threshold = script_effect_value(world, ctx.caster, EMBRACE, 1);
    let health_pct = health.current / health.max * 100.0;
    if health_pct >= threshold {
        return 1.0;
    }
    let amount = script_effect_value(world, ctx.caster, EMBRACE, 0) / 100.0;
    1.0 + amount * (1.0 - health_pct / threshold)
}
