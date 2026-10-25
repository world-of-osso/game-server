//! Canonical actor/session components shared by headless gameplay and native hosts.
use bevy::prelude::*;

/// Links a connection entity to its spawned player entity.
#[derive(Component, Debug)]
pub struct PlayerLink(pub Entity);

/// The entity this player is currently targeting in combat.
#[derive(Component, Debug, Default)]
pub struct CombatTarget(pub Option<Entity>);

/// Whether the player's newest input requests translation. Server-only; a
/// blocking cast freezes `Position`, so casts read intent instead.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Moving(pub bool);

/// Marks an entity for deferred despawn (next frame), allowing lightyear to process
/// the removal of Replicate/NetworkVisibility before the entity disappears.
#[derive(Component)]
pub struct PendingDespawn;

/// Marker for authenticated connections.
#[derive(Component, Debug)]
pub struct Authenticated {
    pub account_id: u64,
}

/// Marker for connections that have entered the world.
#[derive(Component, Debug)]
pub struct InWorld {
    pub character_id: u64,
}

/// Marker component linking a player entity to its character_id for periodic saves.
#[derive(Component, Debug)]
pub struct ClientId(pub u64);
