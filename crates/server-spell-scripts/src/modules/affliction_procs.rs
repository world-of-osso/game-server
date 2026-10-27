//! SimC76f8118 sc_warlock_actions.cpp: Cunning PRD and critical Ravenous RPPM.
use super::combat;
use bevy::prelude::*;
use combat::auras::{ActiveAuras, AuraApplication, AuraSource, procs};
use combat::class_progression::TraitLoadout;
use combat::spell_cast::{
    self,
    effects::EffectCtx,
    script_registry::{EnemyContextHitHook, SpellScriptHook},
};

const CUNNING: u32 = 453172;
const VOLLEY: u32 = 453176;
const RAVENOUS: u32 = 459440;
const NIGHTFALL: u32 = 264571;
const DRAIN_SOUL: u32 = 198590;
const GRASP: u32 = 1261153;
// SimC76f8118 sc_warlock.hpp1066–1067/init1232–1235: selected-build
// PRD behavior. Native453172 effect1 is the enemy cap, not this chance.
const CUNNING_SHADOW_BOLT_CHANCE: f64 = 0.50;
const CUNNING_DRAIN_SOUL_CHANCE: f64 = 0.25;
const RAVENOUS_DOTS: &[u32] = &[980, 146739, 445474, 1259790];
const RAVENOUS_EXTRA_TICKS: &[u32] = &[1261166, 1261176, 1261158, 1279686];

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::EnemyContextHit(bolt_hit),
    SpellScriptHook::PeriodicPrimaryDamage(periodic_hit),
    SpellScriptHook::AfterPolicyDirectDamage(extra_tick_hit),
    SpellScriptHook::OwnsPolicyProc(owns_ravenous),
];

#[derive(Component)]
struct CunningAttempts {
    passive: u32,
    count: u32,
}

fn owned_passive(world: &World, caster: Entity, spell: u32) -> Option<u32> {
    world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.caster == caster && aura.spell_id == spell)
        .map(|aura| aura.instance_id)
}

// Source actions887–898: the native direct-hit boundary has already rolled
// Shard Instability before this impact callback attempts Cunning.
fn bolt_hit(spell: u32) -> Option<EnemyContextHitHook> {
    (spell == 686).then_some(roll_cunning)
}

// Source actions459–467/2192–2214/2347–2360: Instability's earlier
// primary-tick callback precedes Cunning; Grasp's later callback emits extras.
fn periodic_hit(world: &mut World, ctx: &EffectCtx<'_>, target: Entity, _: f32, critical: bool) {
    if matches!(ctx.spell.id, GRASP | DRAIN_SOUL) {
        roll_cunning(world, ctx, target);
    }
    if critical && RAVENOUS_DOTS.contains(&ctx.spell.id) {
        roll_ravenous(world, ctx.caster);
    }
}

fn roll_cunning(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    let Some(passive) = owned_passive(world, ctx.caster, CUNNING) else {
        return;
    };
    let drain = world.get::<TraitLoadout>(ctx.caster).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|spell| spell.spell_id == DRAIN_SOUL)
    });
    let chance = if drain {
        CUNNING_DRAIN_SOUL_CHANCE
    } else {
        CUNNING_SHADOW_BOLT_CHANCE
    };
    let constant = combat::random::accumulated_proc_constant(chance);
    if roll_cunning_attempt(world, ctx.caster, passive, constant) {
        spell_cast::cast_triggered(world, ctx.caster, VOLLEY, Some(target), ctx.depth + 1);
    }
}

fn roll_cunning_attempt(world: &mut World, caster: Entity, passive: u32, constant: f64) -> bool {
    if world
        .get::<CunningAttempts>(caster)
        .is_none_or(|state| state.passive != passive)
    {
        world
            .entity_mut(caster)
            .insert(CunningAttempts { passive, count: 0 });
    }
    let mut attempts = world.get_mut::<CunningAttempts>(caster).unwrap();
    attempts.count = attempts.count.saturating_add(1);
    let success = combat::random::random::<f64>() < constant * f64::from(attempts.count);
    if success {
        attempts.count = 0;
    }
    success
}

// Source actions2004–2025: each actual critical Grasp extra tick can roll
// Ravenous, independently of the underlying DoT's normal tick stream.
fn extra_tick_hit(world: &mut World, ctx: &EffectCtx<'_>, _: Entity, critical: bool) {
    if critical && RAVENOUS_EXTRA_TICKS.contains(&ctx.spell.id) {
        roll_ravenous(world, ctx.caster);
    }
}

fn owns_ravenous(spell: u32) -> bool {
    spell == RAVENOUS
}

// Source init1227: preserve the selected native RPPM rate/haste/timers;
// only the qualifying critical outcomes above can reach this attempt.
fn roll_ravenous(world: &mut World, caster: Entity) {
    let Some(instance) = owned_passive(world, caster, RAVENOUS) else {
        return;
    };
    if procs::try_native_policy_proc(world, caster, instance) {
        combat::auras::apply_aura(
            world,
            &AuraApplication {
                caster,
                target: caster,
                spell_id: NIGHTFALL,
                effect_mask: u32::MAX,
                source: AuraSource::Cast,
            },
        );
    }
}
