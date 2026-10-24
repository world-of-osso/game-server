//! TC Unit.cpp10678/10694 and Spell.cpp569: scoped invocation counter provenance.
//! The hard-limit/suppression dispatch guards activate only at the atomic cutover.
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use bevy::prelude::*;

type ChainSlots = Arc<Mutex<HashMap<Entity, u32>>>;

#[derive(Resource, Default)]
struct ProcChainLengths(ChainSlots);

pub(crate) struct ProcChainScope {
    slots: ChainSlots,
    unit: Entity,
    previous: Option<u32>,
}

impl Drop for ProcChainScope {
    fn drop(&mut self) {
        let mut slots = self.slots.lock().expect("proc chain slots poisoned");
        match self.previous {
            Some(previous) => {
                slots.insert(self.unit, previous);
            }
            None => {
                slots.remove(&self.unit);
            }
        }
    }
}

pub(crate) fn current(world: &World, unit: Entity) -> u32 {
    world
        .get_resource::<ProcChainLengths>()
        .map_or(0, |lengths| {
            lengths
                .0
                .lock()
                .expect("proc chain slots poisoned")
                .get(&unit)
                .copied()
                .unwrap_or(0)
        })
}

pub(crate) fn enter(
    world: &mut World,
    unit: Entity,
    triggering_length: Option<u32>,
) -> ProcChainScope {
    let slots = Arc::clone(&world.get_resource_or_init::<ProcChainLengths>().0);
    let previous = {
        let mut active = slots.lock().expect("proc chain slots poisoned");
        let previous = active.get(&unit).copied();
        let raised = (previous.unwrap_or(0) + 1).max(triggering_length.unwrap_or(0));
        active.insert(unit, raised);
        previous
    };
    ProcChainScope {
        slots,
        unit,
        previous,
    }
}
