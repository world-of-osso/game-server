//! Native Rip/Rake ticks feed the pinned Thriving Growth accumulator.
#[path = "feral_growth/bursting.rs"]
mod bursting;
use super::combat;
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::random::Rng;
use combat::spell_cast::{effects::EffectCtx, script_registry::SpellScriptHook, targets};
use shared::components::ActiveSpec;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::DeclaredReach(declared_reach),
    SpellScriptHook::EarlyAuraRegistration(bursting::register),
    SpellScriptHook::EnemyContextHit(bursting::enemy_hit),
    // SimC76f8118 sc_druid.cpp13755–13764 explicitly overrides the native
    // stack behavior; every application retains its own native lifetime.
    SpellScriptHook::AsynchronousAuras(&[BLOODSEEKER_VINES]),
    SpellScriptHook::PeriodicPrimaryDamage(roll_growth),
    SpellScriptHook::PeriodicPrimaryDamage(bursting::roll_rampancy),
    SpellScriptHook::ConsumedEffectValues(&[
        (THRIVING_GROWTH, 0),
        (THRIVING_GROWTH, 1),
        (THRIVING_GROWTH, 5),
        (GREEN_THUMB, 0),
        (TWIN_SPROUTS, 0),
        (bursting::RAMPANCY, 0),
        (bursting::RAMPANCY, 2),
    ]),
];
const THRIVING_GROWTH: u32 = 439528;
const GREEN_THUMB: u32 = 1270565;
const BLOODSEEKER_VINES: u32 = 439531;
const TWIN_SPROUTS: u32 = 440117;
const THRESHOLD: f64 = 1000.0;
const RANDOM_SCALE: f64 = 2.0;
const SOURCES: &[(u32, u8)] = &[(1079, 0), (155722, 1)];

fn declared_reach(
    world: &World,
    _caster: Entity,
    spell: &combat::spell_info::SpellInfo,
) -> combat::spell_cast::script_registry::ScriptReach {
    if !SOURCES.iter().any(|(source, _)| *source == spell.id) {
        return Default::default();
    }
    let vines = world
        .resource::<combat::spell_info::SpellStore>()
        .get(BLOODSEEKER_VINES)
        .expect("native Bloodseeker Vines");
    combat::spell_cast::script_registry::ScriptReach::caster(targets::spell_range(vines, true).1)
}

#[derive(Component)]
struct GrowthAccumulator(f64);

fn owned_effect(world: &World, caster: Entity, spell: u32, index: u8) -> Option<f64> {
    let aura = world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.spell_id == spell && aura.caster == caster)?;
    aura.effects
        .iter()
        .find(|effect| effect.index == index)
        .map(|effect| f64::from(effect.amount))
}

fn count_owned_dots(world: &mut World, caster: Entity, spell: u32) -> usize {
    // The scheduler retains an expiring aura while paying its final tick.
    world
        .query::<&ActiveAuras>()
        .iter(world)
        .filter(|auras| {
            auras
                .iter()
                .any(|aura| aura.caster == caster && aura.spell_id == spell)
        })
        .count()
}

// SimC76f8118 sc_druid.cpp12334–12346: native per-source scale, selected
// Green Thumb (Feral only), and native exponent over this caster's active dots.
fn growth_scale(world: &mut World, caster: Entity, spell: u32, index: u8) -> Option<f64> {
    let scale = owned_effect(world, caster, THRIVING_GROWTH, index)?;
    let exponent = owned_effect(world, caster, THRIVING_GROWTH, 5)? / 100.0;
    let green = if world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 103)
    {
        owned_effect(world, caster, GREEN_THUMB, 0).unwrap_or(0.0) / 100.0
    } else {
        0.0
    };
    let count = count_owned_dots(world, caster, spell);
    assert!(
        count > 0,
        "native Growth source retains its ticking owned aura"
    );
    Some(scale * (1.0 + green) / (count as f64).powf(exponent))
}

// SimC76f8118 sc_druid.cpp12357–12399: random initial phase, uniform
// [0,2×scale) increments and one1000-point subtraction per successful tick.
fn advance_accumulator(world: &mut World, caster: Entity, scale: f64) -> bool {
    let mut rng = combat::random::thread_rng();
    if world.get::<GrowthAccumulator>(caster).is_none() {
        world
            .entity_mut(caster)
            .insert(GrowthAccumulator(rng.gen_range(0.0..THRESHOLD)));
    }
    let mut state = world
        .get_mut::<GrowthAccumulator>(caster)
        .expect("Growth state initialized");
    state.0 += rng.gen_range(0.0..scale * RANDOM_SCALE);
    if state.0 < THRESHOLD {
        return false;
    }
    state.0 -= THRESHOLD;
    true
}

fn roll_growth(world: &mut World, ctx: &EffectCtx<'_>, target: Entity, _: f32, _: bool) {
    // SimC76f8118 sc_druid.cpp2416–2448/12400–12406: only actual
    // Rip/Rake damage ticks roll; the proc is cast at that tick's recipient.
    let Some(&(_, index)) = SOURCES.iter().find(|(spell, _)| *spell == ctx.spell.id) else {
        return;
    };
    let Some(scale) = growth_scale(world, ctx.caster, ctx.spell.id, index) else {
        return;
    };
    if advance_accumulator(world, ctx.caster, scale) {
        combat::spell_cast::cast_triggered(world, ctx.caster, BLOODSEEKER_VINES, Some(target), 1);
        apply_twin_sprouts(world, ctx.caster, target);
    }
}

// SimC76f8118 sc_druid.cpp3989–4008: prefer an undotted other target;
// if no other enemy exists, add a second stack to the original target.
fn apply_twin_sprouts(world: &mut World, caster: Entity, target: Entity) {
    let Some(chance) = owned_effect(world, caster, TWIN_SPROUTS, 0) else {
        return;
    };
    if combat::random::thread_rng().gen_range(0.0..100.0) >= chance {
        return;
    }
    let Some(center) = targets::position(world, caster) else {
        return;
    };
    let Some(spell) = world
        .resource::<combat::spell_info::SpellStore>()
        .get(BLOODSEEKER_VINES)
    else {
        return;
    };
    let (_, radius) = targets::spell_range(spell, true);
    let mut enemies =
        targets::area_units(world, caster, center, radius, targets::Relation::Enemy, 0);
    enemies.retain(|&enemy| enemy != target);
    let undotted: Vec<_> = enemies
        .iter()
        .copied()
        .filter(|&enemy| {
            !world.get::<ActiveAuras>(enemy).is_some_and(|auras| {
                auras
                    .iter()
                    .any(|aura| aura.spell_id == BLOODSEEKER_VINES && aura.caster == caster)
            })
        })
        .collect();
    let choices = if undotted.is_empty() {
        &enemies
    } else {
        &undotted
    };
    let recipient = if choices.is_empty() {
        target
    } else {
        choices[combat::random::thread_rng().gen_range(0..choices.len())]
    };
    combat::spell_cast::cast_triggered(world, caster, BLOODSEEKER_VINES, Some(recipient), 1);
}
