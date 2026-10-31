//! Fixture adapter over the production APL.
pub(super) use server_follower_apl::windwalker_rotation::*;
use super::literal_rotation::{
    self,
    execution::Prepared,
};
use crate::simc_apl_native::Encounter;
use bevy::prelude::*;
use facts::WwFacts;
use serde_json::{Value, json};
use server::simc_apl::{Action, ActionFacts};


#[cfg(test)]
mod channel_melee_tests;
#[cfg(test)]
mod dance_breaker_tests;
#[cfg(test)]
mod flurry_tests;
#[cfg(test)]
mod glory_tests;
#[cfg(test)]
mod hero_flurry_tests;
#[cfg(test)]
mod karma_model_tests;
#[cfg(test)]
mod momentum_tests;
#[cfg(test)]
mod source_children_tests;
#[cfg(test)]
mod teachings_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod unbroken_rhythm_tests;
#[cfg(test)]
mod wdp_children_tests;
#[cfg(test)]
mod wdp_duration_tests;
#[cfg(test)]
mod wdp_window_tests;
#[cfg(test)]
mod xuens_battlegear_tests;

pub(super) fn prepare_driver(run: &mut super::Run) -> Result<Box<dyn super::SpecDriver>, String> {
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let mut rotation = Rotation::load(run.app.world_mut(), run.actor, encounter)?;
    super::literal_rotation::capture_fixture(run.app.world_mut(), run.actor, encounter)?;
    while rotation.precombat_step(run.app.world_mut(), run.actor, &run.targets)? {
        super::preparation::finish_pending_setup(run)?;
    }
    rotation.start_combat_at(crate::spell_cast::now(run.app.world()));
    Ok(Box::new(rotation))
}

pub(super) const REPORT_KEY: Option<&str> = Some("windwalker_rotation");

impl super::SpecDriver for Rotation {
    fn label(&self) -> &'static str {
        "LITERAL_SIMC_WINDWALKER_76f8118"
    }

    fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        self.drive(world, actor, targets)
    }

    fn report(&self) -> serde_json::Value {
        self.report()
    }
}
