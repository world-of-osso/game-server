//! Fixture adapter over the production APL.
pub(super) use server_follower_apl::fury_rotation::*;
use super::Run;
use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
use bevy::prelude::*;
use server::simc_apl::{Action, ActionFacts, Facts};
use shared::components::PowerType;


#[cfg(test)]
mod deep_wounds_tests;
#[cfg(test)]
mod mechanics_tests;
#[cfg(test)]
mod sudden_death_tests;
#[cfg(test)]
mod tests;
pub(super) fn prepare_driver(run: &mut Run) -> Result<Box<dyn super::SpecDriver>, String> {
    let encounter = Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let mut rotation = Rotation::load(run.app.world_mut(), run.actor, encounter)?;
    super::literal_rotation::capture_fixture(run.app.world_mut(), run.actor, encounter)?;
    super::preparation::finish_pending_setup(run)?;
    // SimC76f8118 player.cpp6138–6183: source-ordered precombat, not a priority loop.
    loop {
        let prepared = rotation.prepared(
            run.app.world_mut(),
            run.actor,
            &run.targets,
            rotation.driver.snapshot.is_none(),
        )?;
        let Some((action, execution)) = rotation.select(
            run.app.world(),
            run.actor,
            &run.targets,
            "precombat",
            &prepared,
        )?
        else {
            break;
        };
        super::preparation::execute_precombat_plan(run, &mut rotation.driver, action, execution)?;
    }
    rotation.encounter = Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    run.gaps
        .retain(|gap| gap != "NATIVE_ASSISTED_COMBAT_70448_VS_SIMC_APL");
    Ok(Box::new(rotation))
}

pub(super) const REPORT_KEY: Option<&str> = Some("fury_rotation");

impl super::SpecDriver for Rotation {
    fn label(&self) -> &'static str {
        "LITERAL_SIMC_FURY_76f8118"
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
