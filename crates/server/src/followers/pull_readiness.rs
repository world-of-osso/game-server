//! Budget capability, not a cast command or a default-heal substitute.
use bevy::prelude::*;

#[derive(Debug)]
pub(super) struct ReadyHeal {
    pub healer: Entity,
    pub spell_id: u32,
    pub costs: Vec<(crate::spell_cast::CostPool, f32)>,
}

pub(super) fn read_ready_heal(
    _world: &mut World,
    _tank: Entity,
    _party: &[Entity],
) -> Result<Option<ReadyHeal>, String> {
    Ok(None)
}
