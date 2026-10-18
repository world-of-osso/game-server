//! Source presence and formation identity for pull admission, independent of visibility.
use bevy::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

use crate::maps::InMap;

#[derive(Debug)]
pub(super) struct Source {
    pub guid: u32,
    pub entry: u32,
    pub pack_guid: u32,
    pub position: Vec3,
    pub actor: Option<Entity>,
}

pub(super) fn read_present_sources(
    _world: &mut World,
    _map: InMap,
    _difficulty: u32,
    _cleared: &BTreeSet<u32>,
) -> Result<BTreeMap<u32, Source>, String> {
    Ok(BTreeMap::new())
}
