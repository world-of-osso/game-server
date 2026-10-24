//! Explicit fixture contracts shared by authoritative gameplay and external test consumers.
pub mod graph;
pub mod healer_intake;
pub use graph::Space;

/// Explicit fixture controller capture; preserves native Follower/NPC identity.
#[derive(bevy::prelude::Component)]
pub struct LiteralParityFixture;
