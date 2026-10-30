//! Explicit fixture contracts shared by authoritative gameplay and external test consumers.
pub mod flight;
pub mod graph;
pub mod healer_intake;
pub mod objectives;
pub mod transport;
pub use crate::navigation::within_reach;
pub use graph::Space;

/// Install the same player death transitions and release handling as the server.
pub fn install_player_death(app: &mut bevy::prelude::App) {
    app.add_plugins(crate::death::DeathPlugin);
}

pub fn player_death_recorded(world: &bevy::prelude::World, player: bevy::prelude::Entity) -> bool {
    world
        .get::<crate::death::PlayerDeathState>(player)
        .is_some_and(|state| state.state == shared::death::DeathState::Dead)
}

/// Submit the native dungeon release for a dead headless player.
pub fn release_dungeon_player(
    world: &mut bevy::prelude::World,
    player: bevy::prelude::Entity,
) -> Result<(), String> {
    use bevy::prelude::*;
    let position = *world
        .get::<shared::components::Position>(player)
        .ok_or("release player lacks position")?;
    let map = *world
        .get::<crate::maps::InMap>(player)
        .ok_or("release player lacks map")?;
    let mut death = world
        .get::<crate::death::PlayerDeathState>(player)
        .cloned()
        .ok_or("release player lacks death state")?;
    let mut queue = bevy::ecs::world::CommandQueue::default();
    world.resource_scope(|world, maps: Mut<crate::maps::MapStore>| {
        let entry = maps
            .map(map.map_id)
            .filter(|entry| entry.is_dungeon())
            .ok_or("release map is not a dungeon")?;
        let mut commands = Commands::new(&mut queue, world);
        crate::death::release_spirit_in_dungeon(
            player,
            &position,
            &mut death,
            entry,
            &maps,
            &mut commands,
        )?;
        Ok::<(), String>(())
    })?;
    world.entity_mut(player).insert(death);
    queue.apply(world);
    Ok(())
}

/// Load the same vehicle accessory catalog used by the live server.
pub fn read_vehicle_accessories(
    root: &std::path::Path,
    db: &rusqlite::Connection,
) -> Result<Vec<(u32, u32)>, String> {
    let (_, accessories) = crate::vehicle::load_runtime(root, db)?;
    Ok(accessories.entry_pairs())
}

/// Install native vehicle accessory systems in a transport-free simulator.
pub fn install_vehicle_accessories(
    app: &mut bevy::prelude::App,
    root: &std::path::Path,
    db: &rusqlite::Connection,
) -> Result<(), String> {
    let (store, accessories) = crate::vehicle::load_runtime(root, db)?;
    app.insert_resource(store);
    app.insert_resource(accessories);
    app.add_systems(
        bevy::prelude::Update,
        crate::vehicle::accessory::install_accessories,
    );
    app.add_systems(
        bevy::prelude::FixedPostUpdate,
        crate::vehicle::accessory::seat_accessories,
    );
    Ok(())
}

/// Explicit fixture controller capture; preserves native Follower/NPC identity.
#[derive(bevy::prelude::Component)]
pub struct LiteralParityFixture;

/// SimC infinite_health fixture: retain native damage/procs without killing the actor.
#[derive(bevy::prelude::Component)]
pub struct SimcInfiniteHealth;

/// Profile race for an explicit reference fixture; this does not turn an NPC into a Player.
#[derive(bevy::prelude::Component)]
pub struct ReferenceRace(pub u8);

/// Successful Divine Inspiration procs on a fixture actor.
#[derive(bevy::prelude::Component, Default)]
pub struct DivineInspirationProcs(pub u32);

/// Make newly applied aura stats visible before a fixture's next decision.
pub fn recompute_fixture_stats(world: &mut bevy::prelude::World) -> Result<(), String> {
    world
        .run_system_cached(crate::stats::update_character_stats)
        .map_err(|error| error.to_string())
}
