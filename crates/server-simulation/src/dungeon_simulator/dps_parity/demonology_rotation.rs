//! Fixture adapter over the production APL.
use bevy::prelude::*;
use serde_json::Value;
pub(super) use server_follower_apl::demonology_rotation::*;

pub(super) fn prepare_driver(run: &mut super::Run) -> Result<Box<dyn super::SpecDriver>, String> {
    let driver = super::prepare_production_driver(run)?;
    run.gaps
        .retain(|gap| !gap.starts_with("NATIVE_FOLLOWER_KIT_VS_SIMC_APL:"));
    Ok(driver)
}

pub(super) const REPORT_KEY: Option<&str> = Some("demonology_rotation");

impl super::SpecDriver for Rotation {
    fn supports_actor(identity: &crate::followers::simulator::ParityActor) -> bool {
        matches!(identity, crate::followers::simulator::ParityActor::Follower)
    }

    fn label(&self) -> &'static str {
        "LITERAL_SIMC_DEMONOLOGY_76f8118"
    }

    fn report(&self) -> Value {
        self.report()
    }

    fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        self.step(world, actor, targets, false)?;
        Ok(())
    }
}
