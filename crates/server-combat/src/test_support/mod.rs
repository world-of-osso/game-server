//! Explicit fixture contracts shared by authoritative gameplay and external test consumers.
pub mod graph;
pub mod healer_intake;
pub mod transport;
pub use crate::navigation::within_reach;
pub use graph::Space;

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

/// Fresh native Fire Breath release metadata for the proposed APL empower rank.
pub fn fire_breath_release(
    world: &bevy::prelude::World,
    caster: bevy::prelude::Entity,
    rank: u8,
) -> crate::spell_info::SpellInfo {
    crate::spell_cast::script_registry::empower_release_preview(world, caster, 357208, rank)
        .expect("native Fire Breath release script must be registered")
}

/// Explicit fixture controller capture; preserves native Follower/NPC identity.
#[derive(bevy::prelude::Component)]
pub struct LiteralParityFixture;

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
