//! Fixture adapter over the production APL.
pub(super) use server_follower_apl::devourer_rotation::*;

#[cfg(test)]
#[path = "devourer_rotation/facts_tests.rs"]
mod facts_tests;
use bevy::prelude::*;

pub(super) fn prepare_driver(run: &mut super::Run) -> Result<Box<dyn super::SpecDriver>, String> {
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let mut rotation = Rotation::load(run.app.world_mut(), run.actor, encounter)?;
    super::literal_rotation::capture_fixture(run.app.world_mut(), run.actor, encounter)?;
    if run
        .app
        .world()
        .get::<crate::class_progression::KnownSpells>(run.actor)
        .is_some_and(|known| known.contains(1261684))
    {
        // SimC76f8118 sc_demon_hunter.cpp9827–9835 defaults Entropy's precombat souls to effect2.
        let count =
            crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, 1261684, 1);
        for _ in 0..count as usize {
            if !crate::spell_cast::try_cast_triggered(
                run.app.world_mut(),
                run.actor,
                1225789,
                Some(run.actor),
                0,
            ) {
                return Err("Entropy precombat Meta stack cast refused".into());
            }
        }
    }
    while rotation.precombat_step(run.app.world_mut(), run.actor, &run.targets)? {
        super::preparation::finish_pending_setup(run)?;
    }
    rotation.start_combat_at(crate::spell_cast::now(run.app.world()));
    run.gaps.retain(|gap| {
        !gap.starts_with("NATIVE_ASSISTED_COMBAT_70448_VS_SIMC_APL")
            && !gap.starts_with("NATIVE_FOLLOWER_KIT_VS_SIMC_APL")
    });
    Ok(Box::new(rotation))
}

pub(super) const REPORT_KEY: Option<&str> = Some("devourer_rotation");

impl super::SpecDriver for Rotation {
    fn label(&self) -> &'static str {
        "LITERAL_SIMC_DEVOURER_76f8118"
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
