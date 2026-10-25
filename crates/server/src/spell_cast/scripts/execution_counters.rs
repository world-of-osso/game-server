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

struct ConsumedCounter {
    actions: &'static [u32],
    buff: u32,
    required: &'static [u32],
    counter: u32,
}

struct ValuedExecution {
    action: u32,
    passive: u32,
    payload: u32,
    value_effect: u8,
}

const VALUED_EXECUTIONS: &[ValuedExecution] = &[
    // SimC76f8118 sc_warrior.cpp5581–5588/8411/9616: only final Rampage
    // execution grants Frenzy; its selected passive supplies the child's value.
    ValuedExecution {
        action: 201363,
        passive: 335077,
        payload: 335082,
        value_effect: 0,
    },
];

pub(super) fn owns_proc(spell: u32) -> bool {
    VALUED_EXECUTIONS
        .iter()
        .any(|execution| execution.passive == spell)
}

const CONSUMERS: &[ConsumedCounter] = &[
    // SimC76f8118 sc_warrior.cpp4347–4352/4547–4554: Execute consumes
    // one SD stack after base execution, then grants selected Imminent Demise.
    ConsumedCounter {
        actions: &[163201, 280735],
        buff: 52437,
        required: &[29725, 444769],
        counter: 445606,
    },
];

#[derive(Component, Default)]
struct ExecutionCounts(BTreeMap<u32, u32>);

pub(super) fn consume_after_foreground_cast(world: &mut World, caster: Entity, action: u32) {
    for consumer in CONSUMERS
        .iter()
        .filter(|consumer| consumer.actions.contains(&action))
    {
        let Some(instance) = world.get::<ActiveAuras>(caster).and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == consumer.buff && aura.caster == caster)
                .map(|aura| aura.instance_id)
        }) else {
            continue;
        };
        crate::auras::spend_stack(world, caster, instance);
        let selected = world.get::<ActiveAuras>(caster).is_some_and(|auras| {
            consumer
                .required
                .iter()
                .all(|spell| auras.has_spell(*spell))
        });
        if selected
            && !crate::spell_cast::try_cast_triggered(
                world,
                caster,
                consumer.counter,
                Some(caster),
                1,
            )
        {
            warn!(
                ?caster,
                counter = consumer.counter,
                "Native consumed-stack counter refused"
            );
        }
    }
}

pub(super) fn after_launch(world: &mut World, caster: Entity, action: u32) {
    grant_valued_execution_auras(world, caster, action);
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

fn grant_valued_execution_auras(world: &mut World, caster: Entity, action: u32) {
    for execution in VALUED_EXECUTIONS
        .iter()
        .filter(|execution| execution.action == action)
    {
        let value = world.get::<ActiveAuras>(caster).and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == execution.passive && aura.caster == caster)
                .and_then(|aura| {
                    aura.effects
                        .iter()
                        .find(|effect| effect.index == execution.value_effect)
                })
                .map(|effect| effect.amount)
        });
        let Some(value) = value else {
            continue;
        };
        if !crate::spell_cast::cast_triggered_with_overrides(
            world,
            caster,
            execution.payload,
            Some(caster),
            1,
            crate::spell_cast::TriggeredSpellOverrides {
                effect0_amount: Some(value),
                ..Default::default()
            },
        ) {
            warn!(
                ?caster,
                payload = execution.payload,
                "Native valued execution aura refused"
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
