//! SimC76f8118 sc_priest_shadow.cpp598–645/2355–2371: Pain funds apparitions.
use super::{combat, shadow_apparitions};
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::spell_cast::{effects::EffectCtx, script_registry::SpellScriptHook};

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::ResolvedDamage(after_pain)];
const PAIN: u32 = 589;
const TORMENTED: u32 = 391284;
// The pinned accumulator explicitly authors these, not the talent's dummy100 effect.
const INCREMENT: f64 = 0.13;
const DOT_EXPONENT: f64 = 0.9;

#[derive(Component)]
struct TormentedProgress {
    passive: u32,
    accumulated: f64,
}

fn after_pain(world: &mut World, ctx: &EffectCtx<'_>, target: Entity, crit: bool, amount: f32) {
    if ctx.spell.id != PAIN || amount <= 0.0 {
        return;
    }
    let Some(passive) = read_passive_instance(world, ctx.caster) else {
        return;
    };
    let count = count_owned_pain(world, ctx.caster);
    if count == 0 {
        return;
    }
    let critical_factor = if crit { 2.0 } else { 1.0 };
    let maximum = INCREMENT * 2.0 * critical_factor / (count as f64).powf(DOT_EXPONENT);
    if roll_progress(world, ctx.caster, passive, maximum) {
        shadow_apparitions::spawn_apparitions(world, ctx, target);
    }
}

fn read_passive_instance(world: &World, caster: Entity) -> Option<u32> {
    world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.caster == caster && aura.spell_id == TORMENTED)
        .map(|aura| aura.instance_id)
}

pub(super) fn count_owned_pain(world: &mut World, caster: Entity) -> usize {
    // Include the aura paying its final tick before the scheduler removes it.
    world
        .query::<&ActiveAuras>()
        .iter(world)
        .filter(|auras| {
            auras.iter().any(|aura| {
                let owned = aura.caster == caster && aura.spell_id == PAIN;
                owned && aura.effects.iter().any(|effect| effect.period_ms > 0)
            })
        })
        .count()
}

fn roll_progress(world: &mut World, caster: Entity, passive: u32, maximum: f64) -> bool {
    // SimC proc_rng.cpp221–273: random initial state, uniform increment, rollover.
    let reset = world
        .get::<TormentedProgress>(caster)
        .is_none_or(|state| state.passive != passive);
    if reset {
        let accumulated = combat::random::random::<f64>();
        world.entity_mut(caster).insert(TormentedProgress {
            passive,
            accumulated,
        });
    }
    let increment = combat::random::random::<f64>() * maximum;
    let mut state = world.get_mut::<TormentedProgress>(caster).unwrap();
    state.accumulated += increment;
    if state.accumulated < 1.0 {
        return false;
    }
    state.accumulated -= 1.0;
    true
}
