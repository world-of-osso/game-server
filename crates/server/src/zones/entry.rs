//! Shared initial-spawn / same-actor template projection (TC Creature::UpdateEntry).
use super::*;

/// Source addon path data, separate from the active movement generator's cursor.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AddonPathId(pub u32);

pub(crate) struct EntryProjection;

pub(crate) fn read_entry_projection(
    _data: &WorldData,
    _entry: u32,
    _context: &mut CreatureSpawnContext<'_>,
    _placement: &CreatureSpawn,
    _spawn_data: Option<&CreatureSpawn>,
    _rng: &mut impl rand::Rng,
) -> Option<EntryProjection> {
    Some(EntryProjection)
}

impl EntryProjection {
    pub(crate) fn apply(self, _actor: EntityWorldMut<'_>, _initial_spawn: bool) {}
}
