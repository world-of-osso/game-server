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
    SpellScriptHook::NativeAfterCast(after_cast),
    SpellScriptHook::ResolvedDamage(after_damage),
];

#[derive(Component, Default)]
struct TargetProcDeadlines(HashMap<Entity, f64>);

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    (spell == FELBLADE).then_some(cast_felblade_child)
}

fn cast_felblade_child(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !has_talent(world, caster, FELBLADE) {
        return;
    }
    let Some(target) = spell.cast_primary_target else {
        error!("native Felblade lacks its selected primary");
        return;
    };
    combat::spell_cast::cast_triggered(world, caster, FELBLADE_DAMAGE, Some(target), 1);
}

fn after_damage(world: &mut World, ctx: &EffectCtx, target: Entity, _crit: bool, amount: f32) {
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
