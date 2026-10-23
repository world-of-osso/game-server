//! Literal pinned Windwalker fixture driver; Stockade retains its native kit.
use crate::simc_apl_native::Encounter;
use bevy::prelude::*;
use server::simc_apl::Action;

pub(super) struct Rotation;

impl Rotation {
    pub fn load(_world: &mut World, _actor: Entity, _encounter: Encounter) -> Result<Self, String> {
        Err("literal native Windwalker fixture driver not implemented".into())
    }

    pub fn drive(
        &mut self,
        _world: &mut World,
        _actor: Entity,
        _targets: &[Entity],
    ) -> Result<(), String> {
        Err("literal native Windwalker decision not implemented".into())
    }

    pub fn value_for(
        &self,
        _world: &World,
        _actor: Entity,
        _targets: &[Entity],
        _action: &Action,
        _name: &str,
    ) -> Result<f64, String> {
        Err("native Windwalker action facts not implemented".into())
    }
}

#[cfg(test)]
mod tests;
