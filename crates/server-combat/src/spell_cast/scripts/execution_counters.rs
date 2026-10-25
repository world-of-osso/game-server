//! Native execution-count proc sources, including background executions.
use bevy::prelude::*;
use std::collections::BTreeMap;

use crate::auras::ActiveAuras;

struct ExecutionCounter {
    action: u32,
    passive: u32,
    required: u32,
    threshold_effect: u8,
    buff: u32,
    resets: &'static [u32],
}

const COUNTERS: &[ExecutionCounter] = &[
    // SimC76f8118 sc_warrior.cpp2428–2435: each Slayer execution counts;
    // native Imminent Demise effect1 sets the threshold; grant SD and reset Execute.
    ExecutionCounter {
        action: 445579,
        passive: 444769,
        required: 29725,
        threshold_effect: 0,
        buff: 52437,
        resets: &[163201, 280735],
    },
];

#[derive(Component, Default)]
struct ExecutionCounts(BTreeMap<u32, u32>);

pub(super) fn after_launch(world: &mut World, caster: Entity, action: u32) {
    for counter in COUNTERS.iter().filter(|counter| counter.action == action) {
        let Some(threshold) = read_threshold(world, caster, counter) else {
            continue;
        };
        if advance_counter(world, caster, counter.passive, threshold) {
            super::foreground_procs::grant_buff_and_reset_actions(
                world,
                caster,
                counter.buff,
                counter.resets,
            );
        }
    }
}

fn read_threshold(world: &World, caster: Entity, counter: &ExecutionCounter) -> Option<u32> {
    let auras = world.get::<ActiveAuras>(caster)?;
    if !auras.has_spell(counter.required) {
        return None;
    }
    let passive = auras
        .iter()
        .find(|aura| aura.spell_id == counter.passive && aura.caster == caster)?;
    let threshold = passive
        .effects
        .iter()
        .find(|effect| effect.index == counter.threshold_effect)
        .expect("native execution-counter threshold effect")
        .amount as u32;
    assert!(
        threshold > 0,
        "native execution-counter threshold must be positive"
    );
    Some(threshold)
}

fn advance_counter(world: &mut World, caster: Entity, passive: u32, threshold: u32) -> bool {
    let mut entity = world.entity_mut(caster);
    let mut counts = entity.entry::<ExecutionCounts>().or_default().into_mut();
    let count = counts.0.entry(passive).or_default();
    *count += 1;
    if *count < threshold {
        return false;
    }
    *count = 0;
    true
}
