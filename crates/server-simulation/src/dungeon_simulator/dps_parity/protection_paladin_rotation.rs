//! Prot Paladin fixture adapter over the production APL.
use super::{Run, SpecDriver, literal_rotation::PrecombatPoll};
use crate::simc_apl_native::{Encounter, FightStyle};
use bevy::prelude::*;
use serde_json::{Value, json};
pub(super) use server_follower_apl::protection_paladin_rotation::Rotation;

pub(super) const REPORT_KEY: Option<&str> = Some("protection_rotation");

pub(super) fn prepare_driver(run: &mut Run) -> Result<Box<dyn SpecDriver>, String> {
    let encounter = Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: FightStyle::Patchwerk,
    };
    let mut rotation = Rotation::load(run.app.world_mut(), run.actor, encounter, &run.profile)?;
    for _ in 0..super::preparation::MAX_SETUP_TICKS {
        match rotation.poll_precombat(run.app.world_mut(), run.actor, &run.targets)? {
            PrecombatPoll::Waiting => super::preparation::finish_pending_setup(run)?,
            PrecombatPoll::Selected { action, execution } => {
                super::preparation::execute_precombat_plan(
                    run,
                    &mut rotation.driver,
                    action,
                    execution,
                )?
            }
            PrecombatPoll::Finished => {
                server_follower_apl::SpecRotation::start_combat_at(
                    &mut rotation,
                    crate::spell_cast::now(run.app.world()),
                );
                return Ok(Box::new(rotation));
            }
        }
    }
    Err("PROTECTION_PRECOMBAT_EXCEEDED_30_SECONDS".into())
}

impl SpecDriver for Rotation {
    fn supports_actor(identity: &crate::followers::simulator::ParityActor) -> bool {
        matches!(
            identity,
            crate::followers::simulator::ParityActor::Player { .. }
        )
    }
    fn label(&self) -> &'static str {
        "LITERAL_SIMC_PROTECTION_76f8118"
    }
    fn tank_active_mitigation_auras(&self) -> Option<&'static [u32]> {
        Some(&[132403])
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
