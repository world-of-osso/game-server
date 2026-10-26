//! SimC76f8118 shadow626–638/2052–2084/2322–2339: actual Pain ticks earn Insight.
use super::{combat, shadow_torment};
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::ResolvedDamage(after_pain_tick),
    SpellScriptHook::AfterForegroundSpecial(consume_insight),
    SpellScriptHook::CastTimeProcs(&[INSIGHT]),
    SpellScriptHook::ConsumedEffectValues(&[(DARK_THOUGHTS, 1)]),
];
const PAIN: u32 = 589;
const TALENT: u32 = 375888;
const INSIGHT: u32 = 375981;
const DARK_THOUGHTS: u32 = 1240388;
const INCREMENT: f64 = 0.08;
const DOT_EXPONENT: f64 = 0.9;

#[derive(Component)]
struct InsightProgress {
    passive: u32,
    accumulated: f64,
}

fn after_pain_tick(world: &mut World, ctx: &EffectCtx<'_>, _: Entity, _: bool, amount: f32) {
    if ctx.spell.id != PAIN || ctx.spell.cast_periodic_ordinal.is_none() || amount <= 0.0 {
        return;
    }
    let Some(passive) = read_passive_instance(world, ctx.caster) else {
        return;
    };
    let dots = shadow_torment::count_owned_pain(world, ctx.caster);
    if dots == 0 {
        return;
    }
    let maximum = read_increment(world, ctx.caster) * 2.0 / (dots as f64).powf(DOT_EXPONENT);
    if roll_progress(world, ctx.caster, passive, maximum) {
        // Native375981 owns instant cast, extra maximum charge and use consumption.
        spell_cast::cast_triggered(world, ctx.caster, INSIGHT, Some(ctx.caster), ctx.depth + 1);
    }
}

fn consume_insight(world: &mut World, caster: Entity, spell: &combat::spell_info::SpellInfo) {
    if !matches!(spell.id, 8092 | 450983) {
        return;
    }
    let instance = world.get::<ActiveAuras>(caster).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.caster == caster && aura.spell_id == INSIGHT)
            .map(|aura| aura.instance_id)
    });
    if let Some(instance) = instance {
        // SimC sc_priest.cpp298–316: execute owns one live Insight use.
        // Native aura500 already skips the underlying charge consumption.
        combat::auras::spend_stack(world, caster, instance);
    }
}

fn read_passive_instance(world: &World, caster: Entity) -> Option<u32> {
    world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.caster == caster && aura.spell_id == TALENT)
        .map(|aura| aura.instance_id)
}

fn read_increment(world: &World, caster: Entity) -> f64 {
    let bonus = if combat::auras::has_aura(world, caster, DARK_THOUGHTS) {
        f64::from(spell_cast::scripts::script_effect_value(
            world,
            caster,
            DARK_THOUGHTS,
            1,
        )) / 100.0
    } else {
        0.0
    };
    // SimC2326–2338 explicitly authors8%, with native selected Dark Thoughts.
    INCREMENT * (1.0 + bonus)
}

fn roll_progress(world: &mut World, caster: Entity, passive: u32, maximum: f64) -> bool {
    // proc_rng.cpp221–273: independent random initial phase and rollover.
    let reset = world
        .get::<InsightProgress>(caster)
        .is_none_or(|state| state.passive != passive);
    if reset {
        let accumulated = combat::random::random::<f64>();
        world.entity_mut(caster).insert(InsightProgress {
            passive,
            accumulated,
        });
    }
    let increment = combat::random::random::<f64>() * maximum;
    let mut state = world.get_mut::<InsightProgress>(caster).unwrap();
    state.accumulated += increment;
    if state.accumulated < 1.0 {
        return false;
    }
    state.accumulated -= 1.0;
    true
}
