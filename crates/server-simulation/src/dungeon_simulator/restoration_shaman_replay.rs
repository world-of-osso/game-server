//! Restoration Shaman fixture policy over the production healer driver.
#[cfg(test)]
#[path = "restoration_shaman_replay_tests.rs"]
mod tests;
use super::healer_simc_replay::Policy;
use crate::simc_apl_native::{Encounter, FightStyle};
use bevy::prelude::{Entity, World};
pub(super) use server_follower_apl::restoration_shaman_rotation::*;

pub(super) fn prepare(
    run: &mut super::dps_parity::Run,
    healer: Entity,
) -> Result<Box<dyn Policy>, String> {
    let encounter = Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: FightStyle::Patchwerk,
    };
    let policy = Restoration::load(run.app.world_mut(), healer, encounter)?;
    super::dps_parity::literal_rotation::capture_fixture(run.app.world_mut(), healer, encounter)?;
    Ok(Box::new(FirstTarget {
        policy,
        enemy: run.targets[0],
    }))
}

/// The fixture damages its first dummy whenever no heal is due.
struct FirstTarget {
    policy: Restoration,
    enemy: Entity,
}

impl Policy for FirstTarget {
    fn drive(&mut self, world: &mut World, healer: Entity, party: &[Entity]) -> Result<(), String> {
        self.policy.drive(world, healer, party, Some(self.enemy))
    }
}
