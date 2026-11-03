//! Fixture adapter over the production APL.
use super::Run;
#[cfg(test)]
use crate::simc_apl_native::Encounter;
use crate::simc_apl_native::{Bindings, NativeFacts};
use bevy::prelude::*;
use server::simc_apl::{Action, ActionFacts, Facts};
pub(super) use server_follower_apl::fury_rotation::*;
use shared::components::PowerType;

#[cfg(test)]
mod deep_wounds_tests;
#[cfg(test)]
mod mechanics_tests;
#[cfg(test)]
mod sudden_death_tests;
#[cfg(test)]
mod tests;
pub(super) fn prepare_driver(run: &mut Run) -> Result<Box<dyn super::SpecDriver>, String> {
    let driver = super::prepare_production_driver(run)?;
    run.gaps
        .retain(|gap| gap != "NATIVE_ASSISTED_COMBAT_70448_VS_SIMC_APL");
    Ok(driver)
}

pub(super) const REPORT_KEY: Option<&str> = Some("fury_rotation");

impl super::SpecDriver for Rotation {
    fn label(&self) -> &'static str {
        "LITERAL_SIMC_FURY_76f8118"
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
