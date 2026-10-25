//! SimC76f8118 Marksmanship relationships absent from native spell edges.
use bevy::prelude::*;
use shared::components::ActiveSpec;

use super::super::{PlannedEffect, TargetPlan, effects::EffectCtx, targets};
use super::cast_relationships::{decrement, has_talent};
use crate::auras::{self, AuraInstance};
use crate::spell_info::{SpellInfo, SpellStore};

const SPEC: u32 = 254;
const AIMED_SHOT: u32 = 19434;
const RAPID_FIRE_SHOT: u32 = 257045;
const TRICK_SHOTS: u32 = 257621;
const TRICK_BUFF: u32 = 257622;
const VOLLEY: u32 = 260243;
const MASTER_MARKSMAN: u32 = 260309;
const MASTER_BLEED: u32 = 269576;
const BLOOD_FLETCHING: u32 = 1319011;

pub(super) fn selected(world: &World, caster: Entity, talent: u32) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == SPEC)
        && has_talent(world, caster, talent)
}

/// sc_hunter.cpp5315–5318/5371–5374 and5629–5647: primary is full
/// damage; native Trick Shots effect0/2 caps extras, effect3/4 scales extras.
pub(super) fn trick_plan(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    explicit: Option<Entity>,
) -> TargetPlan {
    let primary = explicit.expect("Trick Shots action requires its actual primary");
    let mut units = vec![primary];
    let mut chain_multipliers = Vec::new();
    if selected(world, caster, TRICK_SHOTS) && auras::has_aura(world, caster, TRICK_BUFF) {
        let (count_index, percent_index) = if spell.id == AIMED_SHOT {
            (0, 3)
        } else {
            (2, 4)
        };
        let count = super::script_effect_value(world, caster, TRICK_SHOTS, count_index) as usize;
        let multiplier =
            super::script_effect_value(world, caster, TRICK_SHOTS, percent_index) / 100.0;
        let extras = select_trick_targets(world, caster, primary, count);
        chain_multipliers.extend(extras.iter().map(|&target| (target, multiplier)));
        units.extend(extras);
    }
    vec![PlannedEffect {
        index: 0,
        units,
        chain_multipliers,
        dest: None,
        selected_units: Vec::new(),
        blocked_units: Vec::new(),
        prevent_hit_script: false,
    }]
}

fn select_trick_targets(
    world: &mut World,
    caster: Entity,
    primary: Entity,
    count: usize,
) -> Vec<Entity> {
    // sc_hunter.cpp5317/5633: 8-yard target-centered splash. Native Volley
    // radius14 supplies that same radius; do not use the ranged cast distance.
    let volley = world
        .resource::<SpellStore>()
        .get(VOLLEY)
        .expect("native Volley radius");
    let radius = targets::effect_radius(&volley.effects[0]);
    let center = targets::position(world, primary).expect("actual primary position");
    let mut extras =
        targets::area_units(world, caster, center, radius, targets::Relation::Enemy, 0);
    extras.retain(|&target| target != primary);
    // sc_hunter.cpp5393–5396/5665–5668 randomizes only when exceeding the cap.
    if extras.len() > count {
        use crate::random::seq::SliceRandom;
        extras.shuffle(&mut crate::random::thread_rng());
        extras.truncate(count);
    }
    extras
}

/// sc_hunter.cpp5833–5836/3632–3637: consume after the last Rapid tick,
/// never at channel launch, and never while the actual Volley is active.
pub(crate) fn finish_rapid(world: &mut World, caster: Entity, _aura: &AuraInstance) {
    if selected(world, caster, TRICK_SHOTS) && !auras::has_aura(world, caster, VOLLEY) {
        decrement(world, caster, TRICK_BUFF);
    }
}

/// sc_hunter.cpp8583–8626/5175–5178: actual critical damage supplies the
/// residual pool, with Blood Fletching's native share for Aimed Shot only.
pub(in crate::spell_cast) fn apply_residual(
    world: &mut World,
    ctx: &EffectCtx,
    target: Entity,
    critical: bool,
    damage: f32,
) {
    if !critical
        || damage <= 0.0
        || ctx.spell.id == MASTER_BLEED
        || !selected(world, ctx.caster, MASTER_MARKSMAN)
    {
        return;
    }
    let hunter_action = ctx
        .spell
        .class_options
        .as_ref()
        .is_some_and(|options| options.spell_class_set == 9);
    if !hunter_action {
        return;
    }
    let talent = if ctx.spell.id == AIMED_SHOT && has_talent(world, ctx.caster, BLOOD_FLETCHING) {
        BLOOD_FLETCHING
    } else {
        MASTER_MARKSMAN
    };
    let percent = super::script_effect_value(world, ctx.caster, talent, 0);
    bank_residual(world, ctx, target, MASTER_BLEED, percent, damage);
}

pub(super) fn bank_residual(
    world: &mut World,
    ctx: &EffectCtx,
    target: Entity,
    child: u32,
    percent: f32,
    damage: f32,
) {
    let tick_amount = damage * percent / 100.0 / native_residual_ticks(world, child) as f32;
    super::super::cast_triggered_with_effect0_amount(
        world,
        ctx.caster,
        child,
        Some(target),
        ctx.depth + 1,
        tick_amount,
    );
}

fn native_residual_ticks(world: &World, child: u32) -> u32 {
    let bleed = world
        .resource::<SpellStore>()
        .get(child)
        .expect("native Master Marksman bleed");
    let duration = bleed
        .misc
        .as_ref()
        .and_then(|misc| misc.duration)
        .expect("native residual duration")
        .duration_ms;
    let period = bleed.effects[0].aura_period_ms;
    assert!(
        duration > 0 && period > 0,
        "native Master Marksman requires periodic timing"
    );
    let ticks = duration as u32 / period;
    assert!(
        ticks > 0,
        "native Master Marksman requires at least one tick"
    );
    ticks
}

/// sc_hunter.cpp5686–5687: one native Take Aim reduction per executed
/// Rapid tick, not once per recipient and not only on critical impacts.
pub(crate) fn begin_rapid_tick(world: &mut World, caster: Entity) {
    const TAKE_AIM: u32 = 1273132;
    if selected(world, caster, TAKE_AIM) {
        let seconds = f64::from(super::script_effect_value(world, caster, TAKE_AIM, 1)) / 1000.0;
        super::reduce_cooldown(world, caster, AIMED_SHOT, seconds);
    }
}

/// sc_hunter.cpp3991/4014–4016: native talent chance on actual Auto Shot.
pub(super) fn roll_auto_lock(world: &mut World, ctx: &EffectCtx) {
    const LOCK_AND_LOAD: u32 = 194595;
    if ctx.spell.id != 75 || !selected(world, ctx.caster, LOCK_AND_LOAD) {
        return;
    }
    let chance = super::script_effect_value(world, ctx.caster, LOCK_AND_LOAD, 0);
    if crate::random::random::<f32>() * 100.0 < chance {
        super::super::cast_triggered(world, ctx.caster, 194594, Some(ctx.caster), 1);
    }
}

pub(super) fn target_plan(spell: u32) -> Option<(targets::Relation, super::TargetPlanHook)> {
    matches!(spell, AIMED_SHOT | RAPID_FIRE_SHOT).then_some((targets::Relation::Enemy, trick_plan))
}
