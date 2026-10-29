//! Explicit fixture contracts shared by authoritative gameplay and external test consumers.
pub mod graph;
pub mod healer_intake;
pub use crate::navigation::within_reach;
pub use graph::Space;

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
