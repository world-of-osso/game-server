//! Generic creature-melee projection, using native stats/rolls rather than kit defaults.
use super::*;

#[derive(Debug)]
pub(crate) struct MeleeEstimate {
    pub expected_hit: f64,
    pub largest_hit: f64,
    pub interval_secs: f64,
    pub next_swing_secs: f64,
    pub remaining_health: f64,
    pub level: u8,
    pub rank: u8,
}

pub(crate) fn read_generic_creature(
    _world: &World,
    _enemy: Entity,
    _tank: Entity,
) -> Result<MeleeEstimate, String> {
    Err("generic creature forecast is not wired".into())
}
