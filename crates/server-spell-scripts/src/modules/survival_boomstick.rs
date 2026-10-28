//! Boomstick uses earned Tip through the existing Hunter consumers.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, AuraInstance, RemoveReason, scripts::AuraScripts};
use combat::spell_cast::{
    self, script_registry::SpellScriptHook, scripts::script_effect_value, targets,
};
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::BeforeProcLaunch(spend_tip),
    SpellScriptHook::ChannelPeriodic(periodic_tick),
    SpellScriptHook::BeforeNativeOnCast(snapshot_shellshock_targets),
    SpellScriptHook::TargetDamageMultiplier(shellshock_multiplier),
    SpellScriptHook::LateAuraRegistration(register),
];

const BOOMSTICK: u32 = 1261193;
const TIP: u32 = 260286;
const BOOMSTICK_TIP: u32 = 471536;
const BOOMSTICK_DAMAGE: u32 = 1261215;
const SHELLSHOCK: u32 = 1252931;
const MONGOOSE_ROUNDS: u32 = 1253945;
const MONGOOSE_FURY: u32 = 259388;
const SURVIVAL_FOUR_PIECE: u32 = 1296635;

fn periodic_tick(
    spell: u32,
    effect: u8,
) -> Option<combat::spell_cast::script_registry::ContextHitHook> {
    (spell == BOOMSTICK && effect == 0).then_some(emit_tick)
}

fn emit_tick(world: &mut World, ctx: &combat::spell_cast::effects::EffectCtx<'_>, unit: Entity) {
    // SimC76f8118 sc_hunter.cpp6333–6341: emit the existing native
    // periodic child first; then grant selected stacks once per channel tick.
    let child = ctx
        .spell
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Boomstick periodic effect")
        .trigger_spell;
    assert_ne!(child, 0, "native Boomstick periodic child");
    spell_cast::cast_triggered(world, ctx.caster, child, Some(unit), 1);
    if auras::has_aura(world, ctx.caster, SURVIVAL_FOUR_PIECE) {
        // SimC76f8118 sc_hunter.cpp6337–6338 extends all existing async stacks.
        let milliseconds = script_effect_value(world, ctx.caster, SURVIVAL_FOUR_PIECE, 0) as u32;
        auras::extend_owned_async_duration(world, ctx.caster, MONGOOSE_FURY, milliseconds);
    }
    if !super::cast_relationships::has_talent(world, ctx.caster, MONGOOSE_ROUNDS) {
        return;
    }
    let count = script_effect_value(world, ctx.caster, MONGOOSE_ROUNDS, 0) as u8;
    for _ in 0..count {
        super::survival_buffs::grant_self_buff(world, ctx.caster, MONGOOSE_FURY);
    }
}

#[derive(Component)]
struct ShellshockTargets(usize);

fn snapshot_shellshock_targets(world: &mut World, caster: Entity, spell: &SpellInfo, _: u8) {
    if spell.id != BOOMSTICK_DAMAGE
        || !super::cast_relationships::has_talent(world, caster, SHELLSHOCK)
    {
        return;
    }
    // SimC76f8118 sc_hunter.cpp6274–6280 uses this execution's actual
    // recipients. Reuse the native child selector, not global enemy count.
    let effect = spell
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Boomstick damage effect");
    let cap = spell
        .target_restrictions
        .as_ref()
        .map_or(0, |limits| limits.max_targets);
    let units = targets::select_targets(
        world,
        caster,
        spell.cast_primary_target,
        spell,
        effect,
        cap,
        false,
    )
    .expect("native Boomstick has supported target selection");
    world
        .entity_mut(caster)
        .insert(ShellshockTargets(units.len()));
}

fn shellshock_multiplier(world: &World, caster: Entity, _: Entity, spell: &SpellInfo) -> f32 {
    if spell.id != BOOMSTICK_DAMAGE
        || !super::cast_relationships::has_talent(world, caster, SHELLSHOCK)
    {
        return 1.0;
    }
    let count = world
        .get::<ShellshockTargets>(caster)
        .expect("native Boomstick recipient snapshot")
        .0;
    let base = script_effect_value(world, caster, SHELLSHOCK, 0);
    let penalty = script_effect_value(world, caster, SHELLSHOCK, 1);
    let percent = if count > 1 {
        (base - count as f32 * penalty).max(0.0)
    } else {
        base
    };
    1.0 + percent / 100.0
}

fn spend_tip(world: &mut World, caster: Entity, spell: &SpellInfo) {
    // SimC76f8118 sc_hunter.cpp6304–6324: real channel execution spends
    // one earned use, grants its native buff and calls the shared consumers.
    if spell.id != BOOMSTICK || !owns_tip(world, caster) {
        return;
    }
    let Some(target) = spell.cast_primary_target else {
        return;
    };
    super::cast_relationships::decrement(world, caster, TIP);
    spell_cast::cast_triggered(world, caster, BOOMSTICK_TIP, Some(caster), 1);
    super::survival_buffs::after_tip_impact(world, caster);
    super::marksmanship_sentinel::roll_survival_mark(world, caster, target);
    super::survival_pet::strike_after_tip(world, caster, target, 1);
}

fn owns_tip(world: &World, caster: Entity) -> bool {
    world.get::<ActiveAuras>(caster).is_some_and(|auras| {
        auras
            .iter()
            .any(|aura| aura.spell_id == TIP && aura.caster == caster)
    })
}

fn register(scripts: &mut AuraScripts) {
    scripts.0.entry(BOOMSTICK).or_default().on_remove = Some(clear_tip_buff);
}

fn clear_tip_buff(world: &mut World, unit: Entity, aura: &AuraInstance, _: RemoveReason) {
    // sc_hunter.cpp6344–6350: native buff lasts only through the channel.
    if aura.caster == unit {
        auras::remove_auras(world, unit, RemoveReason::Cancelled, |buff| {
            buff.spell_id == BOOMSTICK_TIP && buff.caster == unit
        });
    }
}
