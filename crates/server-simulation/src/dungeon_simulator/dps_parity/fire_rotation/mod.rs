//! Fixture adapter over the production APL.
pub(super) use server_follower_apl::fire_rotation::*;
use super::literal_rotation::{
    self, Driver, PrecombatPoll, equipment::Equipment, execution::Prepared,
};
use crate::simc_apl_native::{Bindings, Encounter};
use bevy::prelude::*;
use facts::FireFacts;
use serde_json::{Value, json};


fn apply_initial_spheres(run: &mut super::Run) {
    let selected = run.app.world().get::<crate::class_progression::KnownSpells>(run.actor)
        .is_some_and(|known| known.contains(448601));
    if selected {
        // SimC76f8118 sc_mage.cpp366/6893–6894: the shipped profile keeps
        // default initial5 applications; the native aura enforces its own stack cap.
        for _ in 0..5 {
            crate::spell_cast::cast_triggered(run.app.world_mut(), run.actor, 448604, Some(run.actor), 1);
        }
    }
}

pub(super) fn prepare_driver(run: &mut super::Run) -> Result<Box<dyn super::SpecDriver>, String> {
    use super::literal_rotation::PrecombatPoll;
    apply_initial_spheres(run);
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let mut rotation = Rotation::load(run.app.world_mut(), run.actor, encounter, &run.profile)?;
    super::literal_rotation::capture_fixture(run.app.world_mut(), run.actor, encounter)?;
    loop {
        match rotation.poll_precombat(run.app.world_mut(), run.actor, &run.targets)? {
            PrecombatPoll::Waiting => super::preparation::finish_pending_setup(run)?,
            PrecombatPoll::Selected { action, execution } => {
                super::preparation::execute_precombat_plan(
                    run,
                    &mut rotation.driver,
                    action,
                    execution,
                )?;
            }
            PrecombatPoll::Finished => break,
        }
    }
    rotation.start_combat_at(crate::spell_cast::now(run.app.world()));
    run.gaps.retain(|gap| {
        !gap.starts_with("NATIVE_ASSISTED_COMBAT_70448_VS_SIMC_APL")
            && !gap.starts_with("NATIVE_FOLLOWER_KIT_VS_SIMC_APL")
    });
    Ok(Box::new(rotation))
}

pub(super) const REPORT_KEY: Option<&str> = Some("fire_rotation");

impl super::SpecDriver for Rotation {
    fn label(&self) -> &'static str {
        "LITERAL_SIMC_FIRE_76f8118"
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
