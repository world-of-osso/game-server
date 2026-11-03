//! Literal pinned Ret fixture adapter over the production APL.
use bevy::prelude::{Entity, World};
pub(super) use server_follower_apl::ret_rotation::*;
// The Ret tests below read these through `use super::*`.
use super::literal_rotation::equipment::Equipment;
use crate::simc_apl_native::Bindings;
use server::simc_apl::{Action, ActionFacts};

#[cfg(test)]
mod context_tests;
#[cfg(test)]
mod dispatch_tests;
#[cfg(test)]
mod equipment_tests;
#[cfg(test)]
mod finisher_tests;
#[cfg(test)]
mod lifecycle_tests;
#[cfg(test)]
mod readiness_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod wrath_gate_tests;

pub(super) fn prepare_driver(run: &mut super::Run) -> Result<Box<dyn super::SpecDriver>, String> {
    super::prepare_production_driver(run)
}

pub(super) const REPORT_KEY: Option<&str> = Some("ret_rotation");

impl super::SpecDriver for Rotation {
    fn label(&self) -> &'static str {
        "LITERAL_SIMC_RETRIBUTION_76f8118"
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
