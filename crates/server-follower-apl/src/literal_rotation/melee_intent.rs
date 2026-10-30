//! Melee intent that the APL reads after native movement abilities displace the actor.
//! SimC76f8118 sc_demon_hunter.cpp1559–1611 returns over distance/run_speed.
use bevy::prelude::*;
use shared::components::MovementControl;

/// The actor walks back toward its auto-attack victim.
#[derive(Component)]
pub struct ReturningToMelee;

/// A displacement ended away from the auto-attack victim; cleared back in melee.
#[derive(Component)]
pub struct DisplacementReturn;

pub fn returning_after_displacement(world: &World, actor: Entity) -> bool {
    world.get::<crate::combat::AutoAttack>(actor).is_some()
        && (forced_movement(world, actor)
            || world.get::<DisplacementReturn>(actor).is_some()
            || world.get::<ReturningToMelee>(actor).is_some())
}

pub fn forced_movement(world: &World, actor: Entity) -> bool {
    world
        .get::<MovementControl>(actor)
        .is_some_and(|control| control.controlled)
        || world
            .get::<crate::spell_cast::ChargeMovement>(actor)
            .is_some()
}
