//! Fixture adapter over the production APL.
use bevy::prelude::*;
pub(super) use server_follower_apl::marksmanship_rotation::*;

pub(super) fn prepare_driver(run: &mut super::Run) -> Result<Box<dyn super::SpecDriver>, String> {
    super::prepare_production_driver(run)
}

pub(super) const REPORT_KEY: Option<&str> = Some("marksmanship_rotation");

impl super::SpecDriver for Rotation {
    fn label(&self) -> &'static str {
        "LITERAL_SIMC_MARKSMANSHIP_76f8118"
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
