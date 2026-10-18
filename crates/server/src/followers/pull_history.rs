//! Test-first observation contract; production event/update wiring follows the RED.
use bevy::prelude::*;
use std::collections::BTreeSet;

#[derive(Debug)]
pub(super) struct Snapshot {
    pub group_dps: Option<f64>,
    pub healer_hps: Option<f64>,
    pub completed_fights: u32,
    pub party_stable: bool,
    pub cleared_spawns: BTreeSet<u32>,
}

pub(super) fn snapshot(_world: &World, _owner: Entity) -> Option<Snapshot> {
    None
}
