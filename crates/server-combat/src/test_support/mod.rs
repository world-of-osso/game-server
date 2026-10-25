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
    crate::spell_cast::scripts::empower_evoker::build_fire_breath_release(world, caster, rank)
}

/// Explicit fixture controller capture; preserves native Follower/NPC identity.
#[derive(bevy::prelude::Component)]
pub struct LiteralParityFixture;

/// Profile race for an explicit reference fixture; this does not turn an NPC into a Player.
#[derive(bevy::prelude::Component)]
pub struct ReferenceRace(pub u8);
