//! Selected Bloodletting applies native Rend from low-health Mortal Strike impacts.
use super::{combat, selected};
use bevy::prelude::*;
use combat::class_progression::TraitLoadout;
use combat::spell_cast::effects::EffectCtx;
use shared::components::Health;

pub(super) fn after_mortal(world: &mut World, ctx: &EffectCtx<'_>, target: Entity, _: bool) {
    if ctx.spell.id != 12294 || !selected(world, ctx.caster, 383154) {
        return;
    }
    let rend_selected = world.get::<TraitLoadout>(ctx.caster).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|talent| talent.spell_id == 772)
    });
    if !rend_selected {
        return;
    }
    // SimC76f8118 sc_warrior.cpp3122–3123: use target health and native
    // effect3 threshold; this also applies to background Unhinged Mortal Strike.
    let threshold = combat::spell_cast::scripts::script_effect_value(world, ctx.caster, 383154, 2);
    let below = world
        .get::<Health>(target)
        .is_some_and(|health| health.max > 0.0 && health.current / health.max * 100.0 < threshold);
    if below
        && !combat::spell_cast::try_cast_triggered(
            world,
            ctx.caster,
            388539,
            Some(target),
            ctx.depth + 1,
        )
    {
        warn!(caster = ?ctx.caster, ?target, "Native Bloodletting Rend refused");
    }
}
