//! Native composition of a projected source footprint with the current run's budget.
use bevy::prelude::*;
use std::collections::BTreeSet;

use super::pull_budget::Decision;

pub(super) fn read_pull_budget(
    _world: &mut World,
    _tank: Entity,
    _projected: &BTreeSet<u32>,
    _difficulty: u32,
) -> Result<Decision, String> {
    Err("Native pull admission is not implemented".into())
}
