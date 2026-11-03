//! Feral fixture adapter over the production APL.
use bevy::prelude::*;
use serde_json::Value;
pub(super) use server_follower_apl::feral_rotation::Rotation;

pub(super) const REPORT_KEY: Option<&str> = Some("feral_rotation");

pub(super) fn prepare_driver(run: &mut super::Run) -> Result<Box<dyn super::SpecDriver>, String> {
    super::prepare_production_driver(run)
}

impl super::SpecDriver for Rotation {
    fn label(&self) -> &'static str {
        "LITERAL_SIMC_FERAL_76f8118"
    }

    fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        Rotation::drive(self, world, actor, targets)
    }

    fn report(&self) -> Value {
        Rotation::report(self)
    }
}
