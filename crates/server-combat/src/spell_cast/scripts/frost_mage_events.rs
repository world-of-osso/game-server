//! Native delayed mage callbacks; read their authored recipient at execution.
use crate::spell_cast;
use bevy::prelude::*;
use std::collections::{BTreeMap, VecDeque};

#[derive(Resource, Default)]
struct Pending(BTreeMap<u64, VecDeque<Request>>);
struct Request {
    caster: Entity,
    event: Event,
}
enum Event {
    Splinters(u32),
    BrainFreeze,
}

fn schedule(world: &mut World, caster: Entity, event: Event, delay_ms: u32) {
    let due = (spell_cast::now(world) * 1000.0) as u64 + u64::from(delay_ms);
    world.init_resource::<Pending>();
    world
        .resource_mut::<Pending>()
        .0
        .entry(due)
        .or_default()
        .push_back(Request { caster, event });
}

pub(super) fn schedule_splinters(world: &mut World, caster: Entity, count: u32, delay_ms: u32) {
    if count > 0 {
        schedule(world, caster, Event::Splinters(count), delay_ms);
    }
}

pub(super) fn schedule_brain_freeze(world: &mut World, caster: Entity) {
    // SimC76f8118 sc_mage.cpp7423–7429: delay only an already active BF refresh.
    schedule(world, caster, Event::BrainFreeze, 150);
}

fn pop_due(world: &mut World, now: u64) -> Option<Request> {
    let mut pending = world.get_resource_mut::<Pending>()?;
    let mut first = pending.0.first_entry()?;
    if *first.key() > now {
        return None;
    }
    let request = first
        .get_mut()
        .pop_front()
        .expect("nonempty native mage callback bucket");
    if first.get().is_empty() {
        first.remove();
    }
    Some(request)
}

pub(in crate::spell_cast) fn resolve(world: &mut World) {
    let now = (spell_cast::now(world) * 1000.0) as u64;
    while let Some(request) = pop_due(world, now) {
        if !spell_cast::targets::is_alive(world, request.caster) {
            continue;
        }
        match request.event {
            Event::Splinters(count) => conjure_current_target(world, request.caster, count),
            Event::BrainFreeze => {
                super::frost_mage_procs::refresh_random_brain_freeze(world, request.caster)
            }
        }
    }
}

fn conjure_current_target(world: &mut World, caster: Entity, count: u32) {
    // sc_mage.cpp7346–7352: the pinned null-target branch explicitly uses
    // mage.target; random target selection is commented out in this revision.
    // This is the authored target, not a fallback for a missing explicit target.
    let Some(target) = world
        .get::<crate::networking::CombatTarget>(caster)
        .and_then(|target| target.0)
    else {
        return;
    };
    super::spellslinger::conjure_count(world, caster, target, count);
}
