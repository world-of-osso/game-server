//! SimC76f8118 sc_monk.cpp4002–4064, 4232–4374 and 6780–6786.
use std::collections::VecDeque;

use bevy::prelude::*;
use shared::components::Health;

use super::combat;
use combat::auras::{AuraInstance, procs::AuraEvent, scripts::AuraScripts};
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::SpellStore;

const SPIRIT: u32 = 400629;
const ORB: u32 = 124506;
const TRIGGER: u32 = 124507;
const EXPIRE: u32 = 178173;
const BLACKOUT_KICK: u32 = 205523;

#[derive(Component, Default)]
struct Orbs(VecDeque<f64>);

#[derive(Component)]
struct ConsumingOrbs;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::EarlyAuraRegistration(register),
    SpellScriptHook::AfterCastUpdate(expire_orbs),
    SpellScriptHook::ExpelHarmHeal(consume_orbs),
    SpellScriptHook::NamedHealingMultiplier(orb_healing_multiplier),
];

fn register(scripts: &mut AuraScripts) {
    let script = scripts.0.entry(SPIRIT).or_default();
    script.check_proc = Some(blackout_kick_only);
    script.on_proc = Some(spawn_orb);
}

fn blackout_kick_only(_: &World, aura: &AuraInstance, event: &AuraEvent) -> bool {
    event.actor == aura.caster && event.spell_id == Some(BLACKOUT_KICK)
}

fn spawn_orb(world: &mut World, actor: Entity, _: &AuraInstance, _: Option<Entity>) {
    let duration = world.resource::<SpellStore>().get(ORB)
        .and_then(|spell| spell.misc.as_ref())
        .and_then(|misc| misc.duration)
        .expect("Gift of the Ox native duration").duration_ms;
    let expires = spell_cast::now(world) + f64::from(duration) / 1000.0;
    let overflow = world.get::<Orbs>(actor).is_some_and(|orbs| orbs.0.len() >= 5);
    if overflow {
        world.get_mut::<Orbs>(actor).unwrap().0.pop_front();
        spell_cast::cast_triggered(world, actor, TRIGGER, Some(actor), 1);
    }
    world.entity_mut(actor).entry::<Orbs>().or_default().get_mut().0.push_back(expires);
}

fn expire_orbs(world: &mut World) {
    let now = spell_cast::now(world);
    let actors: Vec<_> = world.query::<(Entity, &Orbs)>().iter(world)
        .filter(|(_, orbs)| orbs.0.front().is_some_and(|expires| *expires <= now))
        .map(|(actor, _)| actor).collect();
    for actor in actors {
        while world.get::<Orbs>(actor).and_then(|orbs| orbs.0.front()).is_some_and(|expires| *expires <= now) {
            world.get_mut::<Orbs>(actor).unwrap().0.pop_front();
            spell_cast::cast_triggered(world, actor, EXPIRE, Some(actor), 1);
        }
    }
}

fn consume_orbs(world: &mut World, actor: Entity) -> f32 {
    let count = world.get::<Orbs>(actor).map_or(0, |orbs| orbs.0.len());
    let mut effective = 0.0;
    world.entity_mut(actor).insert(ConsumingOrbs);
    for _ in 0..count {
        world.get_mut::<Orbs>(actor).unwrap().0.pop_front();
        let before = world.get::<Health>(actor).expect("orb owner health").current;
        spell_cast::cast_triggered(world, actor, TRIGGER, Some(actor), 1);
        effective += world.get::<Health>(actor).expect("orb owner health").current - before;
    }
    world.entity_mut(actor).remove::<ConsumingOrbs>();
    effective
}

fn orb_healing_multiplier(
    world: &World,
    caster: Entity,
    spell: &combat::spell_info::SpellInfo,
) -> f32 {
    if spell.id != TRIGGER || world.get::<ConsumingOrbs>(caster).is_none() {
        return 1.0;
    }
    let selected = world
        .get::<combat::class_progression::TraitLoadout>(caster)
        .is_some_and(|traits| traits.active_talent_spells().any(|talent| talent.spell_id == 387276));
    if !selected {
        return 1.0;
    }
    let health = world.get::<Health>(caster).expect("orb owner health");
    let missing = 1.0 - (health.current / health.max).max(0.0);
    let percent = combat::spell_cast::scripts::script_effect_value(world, caster, 387276, 0) / 100.0;
    // sc_monk.cpp4336–4346 applies Strength of Spirit only during Expel Harm.
    2.0 + missing * percent
}
