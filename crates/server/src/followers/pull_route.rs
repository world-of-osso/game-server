//! Complete baked route and native source aggro exposure for admission.
use bevy::prelude::*;
use std::collections::BTreeSet;

pub(super) struct Request<'a> {
    pub tank: Entity,
    pub target_guid: u32,
    pub difficulty: u32,
    pub cleared: &'a BTreeSet<u32>,
}

#[derive(Debug)]
pub(super) struct PullRoute {
    pub points: Vec<Vec3>,
    pub spawns: BTreeSet<u32>,
    pub pack_count: usize,
}

pub(super) fn read_route(_world: &mut World, request: &Request<'_>) -> Result<PullRoute, String> {
    let _ = (
        request.tank,
        request.target_guid,
        request.difficulty,
        request.cleared,
    );
    Err("pull route admission is not implemented".into())
}
