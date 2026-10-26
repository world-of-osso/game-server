//! Explicit missing-script relationships; native records retain payload ownership.
use bevy::prelude::*;
use shared::components::ActiveSpec;

use super::combat;
use combat::spell_cast::effects::EffectCtx;
use combat::spell_cast::script_registry::{
    EnemyContextHitHook, Policy, Relationship, SpellScriptHook,
};
use combat::spell_cast::scripts::{AfterCastHook, reduce_cooldown, script_effect_value};
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::Policies(POLICIES),
    SpellScriptHook::PeriodicPrimaryDamage(causality_tick),
    SpellScriptHook::PeriodicPrimaryDamage(scintillation_tick),
    SpellScriptHook::AfterResourceValues(deep_breath_hook),
    SpellScriptHook::EnemyContextHit(surge_hit_hook),
];

fn surge_hit_hook(spell: u32) -> Option<EnemyContextHitHook> {
    (spell == 359077).then_some(shattering_stars)
}

fn shattering_stars(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    const SHATTERING_STARS: u32 = 1265802;
    const STAR_SALVO: u32 = 1265826;
    let devastation = world
        .get::<ActiveSpec>(ctx.caster)
        .is_some_and(|spec| spec.0 == 1467);
    if !devastation || !combat::auras::has_aura(world, ctx.caster, SHATTERING_STARS) {
        return;
    }
    let primary = ctx.spell.cast_primary_target == Some(target);
    if !primary && !combat::auras::has_aura(world, ctx.caster, STAR_SALVO) {
        return;
    }
    let Some(rank) = ctx.spell.cast_empower_level else {
        return;
    };
    // SIMC_DEVASTATION_SHATTERING_STARS: sc_evoker.cpp5550–5573. Native
    // effect0 scales completed rank; Scintillation's actual child effectiveness carries on.
    let rank_percent = script_effect_value(world, ctx.caster, SHATTERING_STARS, 0);
    let effectiveness = (1.0 + f32::from(rank - 1) * rank_percent / 100.0)
        * ctx.spell.cast_damage_multiplier.unwrap_or(1.0);
    let depth = ctx.depth + 1;
    let Some(mut child) = combat::spell_cast::load_triggered_spell(world, 1265804, depth) else {
        return;
    };
    child.cast_damage_multiplier = Some(effectiveness);
    child.cast_damage_role = combat::spell_info::CastDamageRole::Secondary;
    if !combat::spell_cast::launch_triggered_spell(world, ctx.caster, &child, Some(target), depth) {
        warn!(caster = ?ctx.caster, ?target, "Native Shattering Star child refused");
    }
}

fn deep_breath_hook(spell: u32) -> Option<AfterCastHook> {
    matches!(spell, 357210 | 433874).then_some(strafing_run)
}

fn strafing_run(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    const STRAFING_RUN: u32 = 1266151;
    const EXTRA_BREATH: u32 = 1266165;
    let devastation = world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 1467);
    if !devastation || !combat::auras::has_aura(world, caster, STRAFING_RUN) {
        return;
    }
    let instance = world
        .get::<combat::auras::ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == EXTRA_BREATH && aura.caster == caster)
                .map(|aura| aura.instance_id)
        });
    // SIMC_DEVASTATION_STRAFING_RUN: sc_evoker.cpp6344–6353/6384–6398.
    // Native aura411 adds the real charge; aura500 preserves its existing recharge.
    if let Some(instance) = instance {
        combat::auras::spend_stack(world, caster, instance);
        return;
    }
    let depth = combat::spell_cast::next_trigger_depth(world);
    if !combat::spell_cast::try_cast_triggered(world, caster, EXTRA_BREATH, Some(caster), depth) {
        warn!(?caster, "Native Strafing Run grant refused");
    }
}

fn scintillation_tick(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    target: Entity,
    _amount: f32,
    _crit: bool,
) {
    const SCINTILLATION: u32 = 370821;
    if ctx.spell.id != 356995 || !combat::auras::has_aura(world, ctx.caster, SCINTILLATION) {
        return;
    }
    // SIMC_DEVASTATION_SCINTILLATION: sc_evoker.cpp6669–6677. Each real
    // recipient tick rolls native effect1; effect0 owns child effectiveness.
    let chance = script_effect_value(world, ctx.caster, SCINTILLATION, 1);
    if combat::random::random::<f32>() * 100.0 >= chance {
        return;
    }
    let effectiveness = script_effect_value(world, ctx.caster, SCINTILLATION, 0) / 100.0;
    let mut child = super::empower_evoker::build_eternity_surge_release(world, ctx.caster, 1);
    child.cast_damage_multiplier = Some(effectiveness);
    child.cast_dont_reset_periodic_timer = true;
    child.cast_damage_role = combat::spell_info::CastDamageRole::Secondary;
    if !combat::spell_cast::launch_triggered_spell(
        world,
        ctx.caster,
        &child,
        Some(target),
        ctx.depth + 1,
    ) {
        warn!(caster = ?ctx.caster, ?target, "Native Scintillation child refused");
    }
}

fn causality_tick(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    target: Entity,
    _amount: f32,
    _crit: bool,
) {
    const CAUSALITY: u32 = 375777;
    let primary_tick = ctx.spell.id == 356995 && ctx.spell.cast_primary_target == Some(target);
    if !primary_tick {
        return;
    }
    let devastation = world
        .get::<ActiveSpec>(ctx.caster)
        .is_some_and(|spec| spec.0 == 1467);
    if !devastation || !combat::auras::has_aura(world, ctx.caster, CAUSALITY) {
        return;
    }
    // SIMC_DEVASTATION_CAUSALITY: sc_evoker.cpp6694–6699. Native375777
    // effect0 owns the negative millisecond delta; secondary recipients do not repeat it.
    let seconds = -f64::from(script_effect_value(world, ctx.caster, CAUSALITY, 0)) / 1000.0;
    for empower in [357208, 359073] {
        reduce_cooldown(world, ctx.caster, empower, seconds);
    }
}

const POLICIES: &[Policy] = &[
    // SimC76f8118 sc_evoker.cpp6584–6595: one successful Disintegrate
    // spends one earned Mass Disintegrate use, not one per secondary recipient.
    Policy {
        name: "SIMC_DEVASTATION_MASS_DISINTEGRATE_CONSUMER",
        spec: 1467,
        talent: 436335,
        relationship: Relationship::SpendSelfAuraStackAfterCast {
            parent: 356995,
            payload: 436336,
        },
    },
    // SimC76f8118 sc_evoker.cpp6241–6245: Deep Breath's main impact
    // executes native353759; DB2 supplies its periodic damage and recipients.
    Policy {
        name: "SIMC_DEVASTATION_DEEP_BREATH_PAYLOAD",
        spec: 1467,
        talent: 357210,
        relationship: Relationship::TriggerPrimaryAfterCast {
            parent: 357210,
            payload: 353759,
        },
    },
    Policy {
        name: "SIMC_DEVASTATION_MANEUVERABLE_DEEP_BREATH_PAYLOAD",
        spec: 1467,
        talent: 433871,
        relationship: Relationship::TriggerPrimaryAfterCast {
            parent: 433874,
            payload: 353759,
        },
    },
];
