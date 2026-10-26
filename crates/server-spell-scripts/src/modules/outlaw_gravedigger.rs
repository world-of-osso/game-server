//! Native Gravedigger finisher relationships.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras};
use combat::spell_cast::{
    self, effects::EffectCtx, queued_triggers, script_registry::SpellScriptHook,
};
use combat::spell_info::SpellStore;
use shared::components::PowerType;

const DISPATCH: u32 = 2098;
const GRAVEDIGGER_TWO: u32 = 1265862;
const SCOUNDREL_STRIKE: u32 = 1265918;
const GRAVEDIGGER_THREE: u32 = 1265863;
const PALMED_BULLETS: u32 = 1265931;
const GRAVEDIGGER: u32 = 1265935;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::ResourceOnlyContext(&[DISPATCH]),
    SpellScriptHook::ResolvedDamage(after_hit),
];

fn after_hit(world: &mut World, ctx: &EffectCtx, target: Entity, _: bool, damage: f32) {
    if ctx.spell.id != DISPATCH || ctx.depth != 0 || damage <= 0.0 {
        return;
    }
    let points = ctx
        .spell
        .cast_policy_context
        .as_ref()
        .expect("native Dispatch retains effective points")
        .resources
        .iter()
        .find_map(|(power, amount)| (*power == PowerType::ComboPoints).then_some(*amount))
        .expect("native Dispatch has combo-point context");
    earn_palmed_bullet(world, ctx, points);
    if !auras::has_aura(world, ctx.caster, GRAVEDIGGER_TWO) {
        return;
    }
    let threshold = spell_cast::scripts::script_effect_value(world, ctx.caster, GRAVEDIGGER_TWO, 0);
    if points >= threshold {
        // SimC76f8118 sc_rogue.cpp3794/8879–8893: qualifying landed
        // Dispatch launches the native child at its original target after 200ms.
        queued_triggers::schedule_full_trigger(world, ctx, target, SCOUNDREL_STRIKE, 200);
    }
}

fn earn_palmed_bullet(world: &mut World, ctx: &EffectCtx, points: f32) {
    if !auras::has_aura(world, ctx.caster, GRAVEDIGGER_THREE) {
        return;
    }
    let chance = spell_cast::scripts::script_effect_value(world, ctx.caster, GRAVEDIGGER_THREE, 0);
    if combat::random::random::<f32>() * 100.0 >= chance * points {
        return;
    }
    // sc_rogue.cpp8896–8909/10890–10905: landed finishers roll per effective
    // point, and the native Palmed stack threshold earns Gravedigger.
    spell_cast::cast_triggered(
        world,
        ctx.caster,
        PALMED_BULLETS,
        Some(ctx.caster),
        ctx.depth + 1,
    );
    let stacks = world
        .get::<ActiveAuras>(ctx.caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == PALMED_BULLETS && aura.caster == ctx.caster)
        })
        .map_or(0, |aura| aura.stacks);
    let threshold = world
        .resource::<SpellStore>()
        .get(PALMED_BULLETS)
        .expect("native Palmed Bullets spell")
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Palmed Bullets threshold")
        .misc_values[1];
    if i32::from(stacks) == threshold {
        spell_cast::cast_triggered(
            world,
            ctx.caster,
            GRAVEDIGGER,
            Some(ctx.caster),
            ctx.depth + 1,
        );
    }
}
