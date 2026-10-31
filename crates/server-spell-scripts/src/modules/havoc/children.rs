//! SimC76f8118 sc_demon_hunter.cpp7073–7080,7542–7550,7837–7843,7870–7893,11186.
use super::combat;
use super::havoc::has_talent;
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::{effects::EffectCtx, scripts::AfterCastHook};
use combat::spell_info::{SpellInfo, SpellStore};
use std::collections::HashMap;

const FELBLADE: u32 = 232893;
const FELBLADE_DAMAGE: u32 = 213243;
const ESSENCE_BREAK: u32 = 258860;
const MARK: u32 = 320338;
const PROC: u32 = 1245759;
const MARK_DELAY_MS: u32 = 250;
const ELIGIBLE: &[u32] = &[
    199552, 200685, 210153, 210155, 391374, 391378, 393055, 393054, 222031, 199547, 227518, 201428,
];

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::DeclaredReach(declared_reach),
    SpellScriptHook::NativeAfterCast(after_cast),
    SpellScriptHook::ResolvedDamage(after_damage),
];

fn declared_reach(
    world: &World,
    _caster: Entity,
    spell: &SpellInfo,
) -> combat::spell_cast::script_registry::ScriptReach {
    if !matches!(spell.id, 370965 | 370966 | 1246168) {
        return Default::default();
    }
    let store = world.resource::<SpellStore>();
    let radius = [370969, 1246168]
        .into_iter()
        .filter_map(|id| store.get(id))
        .map(|dot| combat::spell_cast::targets::spell_range(dot, true).1)
        .fold(0.0, f32::max);
    combat::spell_cast::script_registry::ScriptReach::caster(radius)
}

#[derive(Component, Default)]
struct TargetProcDeadlines(HashMap<Entity, f64>);

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    (spell == FELBLADE).then_some(cast_felblade_child)
}

fn cast_felblade_child(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !world
        .get::<combat::class_progression::KnownSpells>(caster)
        .is_some_and(|known| known.contains(FELBLADE))
    {
        return;
    }
    let Some(target) = spell.cast_primary_target else {
        error!("native Felblade lacks its selected primary");
        return;
    };
    combat::spell_cast::cast_triggered(world, caster, FELBLADE_DAMAGE, Some(target), 1);
}

fn after_damage(world: &mut World, ctx: &EffectCtx, target: Entity, _crit: bool, amount: f32) {
    if ctx.spell.id == 370966 && amount > 0.0 && has_talent(world, ctx.caster, 370965) {
        apply_hunt_secondary_dots(world, ctx, target, 370969);
    }
    // SimC7524–7551 rolls Onslaught before the eligible Essence Break child.
    super::havoc_onslaught::roll_from_offhand(world, ctx, target, amount);
    if amount <= 0.0 || !has_talent(world, ctx.caster, ESSENCE_BREAK) {
        return;
    }
    if ctx.spell.id == ESSENCE_BREAK {
        combat::spell_cast::queued_triggers::schedule_full_trigger(
            world,
            ctx,
            target,
            MARK,
            MARK_DELAY_MS,
        );
    } else if ELIGIBLE.contains(&ctx.spell.id) {
        proc_marked_strike(world, ctx, target);
    }
}

// SimC76f8118 sc_demon_hunter.cpp5488–5495,5505–5511: Hunt's
// impact applies its native DoT to up to the native370967 effect0 targets.
pub(super) fn apply_hunt_secondary_dots(
    world: &mut World,
    ctx: &EffectCtx,
    primary: Entity,
    dot_id: u32,
) {
    let cap = combat::spell_cast::scripts::script_effect_value(world, ctx.caster, 370967, 0) as u32;
    let Some(center) = combat::spell_cast::targets::position(world, ctx.caster) else {
        error!("Native Hunt caster position absent");
        return;
    };
    let Some(dot) = world.resource::<SpellStore>().get(dot_id) else {
        error!("Native Hunt DoT absent");
        return;
    };
    let range = combat::spell_cast::targets::spell_range(dot, true).1;
    if cap == 0 {
        error!("Native Hunt target cap absent");
        return;
    }
    let targets = combat::spell_cast::targets::area_units(
        world,
        ctx.caster,
        center,
        range,
        combat::spell_cast::targets::Relation::Enemy,
        0,
    );
    for target in targets
        .into_iter()
        .filter(|target| *target != primary)
        .take(cap as usize - 1)
    {
        combat::spell_cast::cast_triggered(world, ctx.caster, dot_id, Some(target), ctx.depth + 1);
    }
}

fn proc_marked_strike(world: &mut World, ctx: &EffectCtx, target: Entity) {
    let now = combat::auras::aura_now(world);
    let marked = world.get::<ActiveAuras>(target).is_some_and(|auras| {
        auras.iter().any(|aura| {
            aura.spell_id == MARK
                && aura.caster == ctx.caster
                && aura.expires_at.is_some_and(|end| end > now)
        })
    });
    if !marked {
        return;
    }
    let ready = world
        .get::<TargetProcDeadlines>(ctx.caster)
        .and_then(|deadlines| deadlines.0.get(&target))
        .is_none_or(|end| *end <= now);
    if !ready {
        return;
    }
    let milliseconds = world
        .resource::<SpellStore>()
        .get(MARK)
        .and_then(|spell| spell.aura_options.as_ref())
        .expect("native Essence Break proc options")
        .proc_category_recovery_ms;
    assert!(
        milliseconds > 0,
        "native Essence Break requires its target ICD"
    );
    world
        .entity_mut(ctx.caster)
        .entry::<TargetProcDeadlines>()
        .or_default()
        .into_mut()
        .0
        .insert(target, now + f64::from(milliseconds) / 1000.0);
    combat::spell_cast::cast_triggered(world, ctx.caster, PROC, Some(target), ctx.depth + 1);
}
