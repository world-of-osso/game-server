//! Fixture adapter over the production APL.
use super::literal_rotation::default_pet::ConfiguredPet;
use crate::simc_apl_native::Encounter;
use bevy::prelude::*;
use serde_json::Value;
pub(super) use server_follower_apl::demonology_rotation::*;

pub(super) fn prepare_driver(run: &mut super::Run) -> Result<Box<dyn super::SpecDriver>, String> {
    let encounter = Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let mut rotation = Rotation::load(run.app.world_mut(), run.actor, &run.profile, encounter)?;
    super::literal_rotation::capture_fixture(run.app.world_mut(), run.actor, encounter)?;
    if let Some(pet) = ConfiguredPet::read(run.app.world(), run.actor, &run.profile)? {
        super::preparation::wait_for_configured_pet_action(run, &pet)?;
    }
    while rotation.step(run.app.world_mut(), run.actor, &run.targets, true)? {
        super::preparation::finish_pending_setup(run)?;
    }
    rotation.start_combat_at(crate::spell_cast::now(run.app.world()));
    run.gaps
        .retain(|gap| !gap.starts_with("NATIVE_FOLLOWER_KIT_VS_SIMC_APL:"));
    Ok(Box::new(rotation))
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
