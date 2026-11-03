//! Fixture adapter over the production APL.
use super::literal_rotation::{self, execution::Prepared};
use crate::simc_apl_native::Encounter;
use bevy::prelude::*;
use facts::WwFacts;
use serde_json::{Value, json};
use server::simc_apl::{Action, ActionFacts};
pub(super) use server_follower_apl::windwalker_rotation::*;

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
    super::prepare_production_driver(run)
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
