//! Explicit source relationships, executed without core spell-ID branches.
//! SIMC_RET_SECOND_SUNRISE_STORM_ECHO: SimC76f8118 Ret495–638.
//! No TC handler/native child edge. Native4442 live talent effects own values/ICD.
//! Parent reports Maxroll12.1.5 and Skycoach15%/30%; native raw15/30 plus
//! specialization1258011 yields20/50. Do not tune native values to web guides.
//! SimC's200ms is TODO/unverified; no artificial delay is used here.
use bevy::prelude::*;

use crate::auras::{ActiveAuras, procs};
use crate::class_progression::{ActiveSpec, TraitLoadout};
use crate::spell_info::SpellInfo;

struct EchoPolicy {
    name: &'static str,
    spec: u32,
    talent: u32,
    root: u32,
    payload: u32,
    chance_effect: u8,
    effectiveness_effect: u8,
}

const POLICIES: &[EchoPolicy] = &[EchoPolicy {
    name: "SIMC_RET_SECOND_SUNRISE_STORM_ECHO",
    spec: 70,
    talent: 431474,
    root: 53385,
    payload: 53385,
    chance_effect: 0,
    effectiveness_effect: 1,
}];

fn selected(world: &World, caster: Entity, policy: &EchoPolicy) -> bool {
    let spec = world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == policy.spec);
    let talent = world.get::<TraitLoadout>(caster).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|spell| spell.spell_id == policy.talent)
    });
    spec && talent
}

fn read_live_values(world: &World, caster: Entity, policy: &EchoPolicy) -> Option<(u32, f32, f32)> {
    let aura = world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.spell_id == policy.talent && aura.caster == caster)?;
    let amount = |index| {
        aura.effects
            .iter()
            .find(|effect| effect.index == index)
            .map(|effect| effect.amount)
    };
    Some((
        aura.instance_id,
        amount(policy.chance_effect)?,
        amount(policy.effectiveness_effect)?,
    ))
}

pub(in crate::spell_cast) fn after_foreground_cast(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
) {
    if spell.cast_parent_owns_modifier_charges {
        return;
    }
    for policy in POLICIES.iter().filter(|policy| policy.root == spell.id) {
        if selected(world, caster, policy) {
            try_echo(world, caster, spell, policy);
        }
    }
}

fn try_echo(world: &mut World, caster: Entity, spell: &SpellInfo, policy: &'static EchoPolicy) {
    let Some(target) = spell.cast_primary_target else {
        warn!(
            policy = policy.name,
            ?caster,
            "Native echo primary unavailable"
        );
        return;
    };
    let Some((instance, chance, effectiveness)) = read_live_values(world, caster, policy) else {
        warn!(
            policy = policy.name,
            ?caster,
            "Native echo effect values unavailable"
        );
        return;
    };
    if !chance.is_finite() || !effectiveness.is_finite() || effectiveness <= 0.0 {
        warn!(
            policy = policy.name,
            chance, effectiveness, "Native echo values invalid"
        );
        return;
    }
    if procs::try_policy_proc(world, caster, instance, chance) {
        queue_echo(world, caster, target, policy, effectiveness / 100.0);
    }
}

fn queue_echo(
    world: &mut World,
    caster: Entity,
    target: Entity,
    policy: &'static EchoPolicy,
    multiplier: f32,
) {
    // Defer after the foreground action without a fabricated authored timer.
    world.commands().queue(move |world: &mut World| {
        let Some(mut child) = super::super::load_triggered_spell(world, policy.payload, 1) else {
            return;
        };
        child.cast_damage_multiplier = Some(multiplier);
        if !super::super::launch_triggered_spell(world, caster, &child, Some(target), 1) {
            warn!(
                policy = policy.name,
                ?caster,
                ?target,
                "Native echo refused"
            );
        }
    });
}
