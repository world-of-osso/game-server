//! Fixture adapter over the production APL.
use bevy::prelude::*;
use serde_json::{Value, json};
pub(super) use server_follower_apl::destruction_rotation::*;

pub(super) fn prepare_driver(run: &mut super::Run) -> Result<Box<dyn super::SpecDriver>, String> {
    let driver = super::prepare_production_driver(run)?;
    run.gaps
        .retain(|gap| !gap.starts_with("NATIVE_FOLLOWER_KIT_VS_SIMC_APL:"));
    Ok(driver)
}

pub(super) const REPORT_KEY: Option<&str> = Some("destruction_rotation");

impl super::SpecDriver for Rotation {
    fn label(&self) -> &'static str {
        "LITERAL_SIMC_DESTRUCTION_76f8118"
    }
    fn report(&self) -> Value {
        json!({"driver": self.label(), "decisions": self.driver.decisions,
            "native_precombat_snapshot": self.driver.snapshot,
            "gaps": ["NATIVE_POTION_INVENTORY_UNAVAILABLE", "EXTERNAL_BUFF_PROVIDER_UNAVAILABLE:power_infusion"]})
    }
    fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        server_follower_apl::SpecRotation::drive(self, world, actor, targets)
    }
}
