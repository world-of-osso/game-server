//! Mistweaver fixture policy over the production healer driver.
#[cfg(test)]
#[path = "mistweaver_mana_tea_tests.rs"]
mod mana_tea_tests;
#[cfg(test)]
#[path = "mistweaver_replay_tests.rs"]
mod tests;
use super::healer_simc_replay::Policy;
use crate::simc_apl_native::{Encounter, FightStyle};
use bevy::prelude::{Entity, World};
pub(super) use server_follower_apl::mistweaver_rotation::*;

pub(super) fn prepare(
    run: &mut super::dps_parity::Run,
    healer: Entity,
) -> Result<Box<dyn Policy>, String> {
    let encounter = Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: FightStyle::Patchwerk,
    };
    let policy = Mistweaver::load(run.app.world_mut(), healer, encounter)?;
    super::dps_parity::literal_rotation::capture_fixture(run.app.world_mut(), healer, encounter)?;
    Ok(Box::new(policy))
}

impl Policy for Mistweaver {
    fn drive(&mut self, world: &mut World, healer: Entity, party: &[Entity]) -> Result<(), String> {
        let enemy = super::healer_simc_replay::tank_target(world, party);
        Mistweaver::drive(self, world, healer, party, enemy)
    }
}
