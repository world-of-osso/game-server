//! Fixture adapter over the production APL.
use bevy::prelude::*;
use serde_json::Value;
pub(super) use server_follower_apl::arcane_rotation::*;

pub(super) fn prepare_driver(run: &mut super::Run) -> Result<Box<dyn super::SpecDriver>, String> {
    let driver = super::prepare_production_driver(run)?;
    run.gaps.retain(|gap| {
        !gap.starts_with("NATIVE_ASSISTED_COMBAT_70448_VS_SIMC_APL")
            && !gap.starts_with("NATIVE_FOLLOWER_KIT_VS_SIMC_APL")
    });
    Ok(driver)
}

pub(super) const REPORT_KEY: Option<&str> = Some("arcane_rotation");

impl super::SpecDriver for Rotation {
    fn label(&self) -> &'static str {
        "LITERAL_SIMC_ARCANE_76f8118"
    }
    fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        self.drive(world, actor, targets)
    }
    fn report(&self) -> Value {
        self.report()
    }
}
