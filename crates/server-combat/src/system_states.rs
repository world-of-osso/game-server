//! Query and system-parameter states that exclusive code reuses across calls instead of
//! rebuilding them over every archetype. They are non-send world data: a resource is an
//! entity in Bevy 0.19, and creating one would shift every later entity ID.
use bevy::ecs::query::{QueryData, QueryFilter};
use bevy::ecs::system::{SystemParam, SystemState};
use bevy::prelude::*;
use std::any::{Any, TypeId};
use std::collections::HashMap;

#[derive(Default)]
struct States(HashMap<TypeId, Box<dyn Any>>);

fn with_cached<T: 'static, R>(
    world: &mut World,
    create: impl FnOnce(&mut World) -> T,
    run: impl FnOnce(&mut World, &mut T) -> R,
) -> R {
    let reused = world
        .get_non_send_mut::<States>()
        .and_then(|mut states| states.0.remove(&TypeId::of::<T>()));
    let mut state: Box<T> = match reused {
        Some(state) => state.downcast().expect("state stored under its own type"),
        None => Box::new(create(world)),
    };
    let result = run(world, &mut state);
    if !world.contains_non_send::<States>() {
        world.insert_non_send(States::default());
    }
    world
        .non_send_mut::<States>()
        .0
        .insert(TypeId::of::<T>(), state);
    result
}

/// Run `run` with this world's `SystemState<P>`, created on first use.
pub fn with_state<P: SystemParam + 'static, R>(
    world: &mut World,
    run: impl FnOnce(&mut World, &mut SystemState<P>) -> R,
) -> R {
    with_cached(world, SystemState::new, run)
}

/// Run `run` with this world's `QueryState<D, F>`, created on first use. A reused query
/// matches archetypes in creation order, unlike a fresh `world.query()`: look units up
/// by entity, or sort what it iterates.
pub fn with_query<D: QueryData + 'static, F: QueryFilter + 'static, R>(
    world: &mut World,
    run: impl FnOnce(&mut World, &mut QueryState<D, F>) -> R,
) -> R {
    with_cached(world, QueryState::new, run)
}

/// Entities matching `F`. `Entity` requires no component, so a reused query walks every
/// archetype in creation order, the same order as a fresh `query_filtered::<Entity, F>()`.
pub fn entities_with<F: QueryFilter + 'static>(world: &mut World) -> Vec<Entity> {
    with_query::<Entity, F, _>(world, |world, query| query.iter(world).collect())
}
