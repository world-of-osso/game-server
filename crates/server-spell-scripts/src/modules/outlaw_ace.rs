//! Ace Up Your Sleeve proc on a landed Between the Eyes.
use super::combat;
use bevy::prelude::*;
use combat::auras;
use combat::random::Rng;
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};
use combat::spell_info::SpellStore;
use shared::components::PowerType;

const BTE: u32 = 315341;
const TALENT: u32 = 381828;
const ENERGIZE: u32 = 394120;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterResolvedHits(after_bte)];

fn after_bte(world: &mut World, ctx: &EffectCtx) {
    if ctx.spell.id != BTE || ctx.depth != 0 || !auras::has_aura(world, ctx.caster, TALENT) {
        return;
    }
    if !spell_cast::resolving_damage(world, ctx)
        .is_some_and(|hits| hits.iter().any(|(_, amount)| *amount > 0.0))
    {
        return;
    }
    let points =
        ctx.spell
            .cast_policy_context
            .as_ref()
            .and_then(|context| {
                context.resources.iter().find_map(|(power, amount)| {
                    (*power == PowerType::ComboPoints).then_some(*amount)
                })
            })
            .expect("Between the Eyes retains its effective combo points");
    let chance = spell_cast::scripts::script_effect_value(world, ctx.caster, TALENT, 0) * points;
    if combat::random::thread_rng().gen_range(0.0..100.0) >= chance {
        return;
    }
    // SimC76f8118 sc_rogue.cpp3852–3859: landed BtE rolls the native
    // per-point chance, earns the native payload and resets the cooldown.
    spell_cast::cast_triggered(world, ctx.caster, ENERGIZE, Some(ctx.caster), ctx.depth + 1);
    let spell = world
        .resource::<SpellStore>()
        .get(BTE)
        .expect("native Between the Eyes")
        .clone();
    let now = spell_cast::now(world);
    let remaining = world
        .get::<spell_cast::SpellHistory>(ctx.caster)
        .map_or(0, |history| history.remaining_cooldown_ms(&spell, now));
    spell_cast::scripts::reduce_cooldown(world, ctx.caster, BTE, remaining as f64 / 1000.0);
}
