//! System-parameter states that exclusive helpers reuse across calls instead of rebuilding
//! their queries over every archetype. They are non-send world data: a resource is an
//! entity in Bevy 0.19, and creating one would shift every later entity ID.
use bevy::ecs::system::{SystemParam, SystemState};
use bevy::prelude::*;
use std::any::{Any, TypeId};
use std::collections::HashMap;

#[derive(Default)]
struct SystemStates(HashMap<TypeId, Box<dyn Any>>);

/// Run `run` with this world's `SystemState<P>`, created on first use. A reused query
/// matches archetypes in creation order, not a fresh query's order: use it for lookups
/// by entity, or sort what it iterates.
pub fn with_state<P: SystemParam + 'static, R>(
    world: &mut World,
    run: impl FnOnce(&mut World, &mut SystemState<P>) -> R,
) -> R {
    let reused = world
        .get_non_send_mut::<SystemStates>()
        .and_then(|mut states| states.0.remove(&TypeId::of::<P>()));
    let mut state = match reused {
        Some(state) => *state
            .downcast::<SystemState<P>>()
            .expect("system state stored under its parameter type"),
        None => SystemState::new(world),
    };
    let result = run(world, &mut state);
    if !world.contains_non_send::<SystemStates>() {
        world.insert_non_send(SystemStates::default());
    }
    world
        .non_send_mut::<SystemStates>()
        .0
        .insert(TypeId::of::<P>(), Box::new(state));
    result
}
