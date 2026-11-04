//! Balance fixture adapter over the production APL.
use bevy::prelude::*;
use server_follower_apl::SpecRotation;
pub(super) use server_follower_apl::balance_rotation::Rotation;

pub(super) const REPORT_KEY: Option<&str> = Some("balance_rotation");

pub(super) fn prepare_driver(run: &mut super::Run) -> Result<Box<dyn super::SpecDriver>, String> {
    super::prepare_production_driver(run)
}

impl super::SpecDriver for Rotation {
    fn label(&self) -> &'static str {
        "LITERAL_SIMC_BALANCE_76f8118"
    }

    fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        SpecRotation::drive(self, world, actor, targets)
    }
}
