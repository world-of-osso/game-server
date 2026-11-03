//! Prot Paladin fixture adapter over the production APL.
use super::{Run, SpecDriver};
use bevy::prelude::*;
use serde_json::{Value, json};
pub(super) use server_follower_apl::protection_paladin_rotation::Rotation;

pub(super) const REPORT_KEY: Option<&str> = Some("protection_rotation");
/// Shield of the Righteous: the native active-mitigation aura replays sample.
pub(crate) const ACTIVE_MITIGATION_AURAS: &[u32] = &[132403];

pub(super) fn prepare_driver(run: &mut Run) -> Result<Box<dyn SpecDriver>, String> {
    super::prepare_production_driver(run)
}

impl SpecDriver for Rotation {
    fn label(&self) -> &'static str {
        "LITERAL_SIMC_PROTECTION_76f8118"
    }
    fn tank_active_mitigation_auras(&self) -> Option<&'static [u32]> {
        Some(ACTIVE_MITIGATION_AURAS)
    }
    fn report(&self) -> Value {
        json!({"driver":self.label(),"decisions":self.driver.decisions})
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
