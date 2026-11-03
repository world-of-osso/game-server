//! Fixture adapter over the production APL.
use bevy::prelude::*;
pub(super) use server_follower_apl::survival_rotation::*;

pub(super) fn prepare_driver(run: &mut super::Run) -> Result<Box<dyn super::SpecDriver>, String> {
    let driver = super::prepare_production_driver(run)?;
    run.gaps
        .retain(|gap| !gap.starts_with("NATIVE_FOLLOWER_KIT_VS_SIMC_APL"));
    Ok(driver)
}

pub(super) const REPORT_KEY: Option<&str> = Some("survival_rotation");

impl super::SpecDriver for Rotation {
    fn label(&self) -> &'static str {
        "LITERAL_SIMC_SURVIVAL_76f8118"
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
