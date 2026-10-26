//! Native Ruthlessness combo points from landed paid Rogue finishers.
use super::combat;
use bevy::prelude::*;
use combat::class_progression::KnownSpells;
use combat::random::Rng;
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};
use combat::spell_info::SpellStore;
use shared::components::PowerType;

const TALENT: u32 = 14161;
const ENERGIZE: u32 = 1258235;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::ResolvedDamage(after_finisher_hit)];

fn after_finisher_hit(world: &mut World, ctx: &EffectCtx, target: Entity, _: bool, damage: f32) {
    if ctx.depth != 0 || damage <= 0.0 || ctx.spell.cast_primary_target != Some(target) {
        return;
    }
    if !world.get::<KnownSpells>(ctx.caster).is_some_and(|known| known.contains(TALENT)) {
        return;
    }
    let Some(points) = ctx.spell.cast_policy_context.as_ref().and_then(|context| {
        context.resources.iter().find_map(|(power, amount)| {
            (*power == PowerType::ComboPoints && *amount > 0.0).then_some(*amount)
        })
    }) else {
        return;
    };
    let chance_per_point = world.resource::<SpellStore>().get(TALENT)
        .expect("known Ruthlessness has native spell data")
        .effects.iter().find(|effect| effect.index == 0)
        .expect("native Ruthlessness chance").points_per_resource;
    // SimC76f8118 sc_rogue.cpp2683–2688/8091–8127: each landed
    // foreground finisher rolls the native chance per effective combo point.
    let chance = chance_per_point * points / 100.0;
    let guaranteed = chance.floor() as u32;
    let extra = combat::random::thread_rng().gen_range(0.0..1.0) < chance.fract();
    for _ in 0..guaranteed + u32::from(extra) {
        spell_cast::cast_triggered(world, ctx.caster, ENERGIZE, Some(ctx.caster), ctx.depth + 1);
    }
}
