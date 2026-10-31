//! Fixture adapter over the production APL.
pub(super) use server_follower_apl::havoc_rotation::*;
use bevy::prelude::*;


pub(super) fn prepare_driver(run: &mut super::Run) -> Result<Box<dyn super::SpecDriver>, String> {
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let mut rotation = Rotation::load(run.app.world_mut(), run.actor, encounter)?;
    super::literal_rotation::capture_fixture(run.app.world_mut(), run.actor, encounter)?;
    run.app
        .world_mut()
        .entity_mut(run.actor)
        .insert(crate::simc_apl_native::FixturePrecombat);
    while rotation.precombat_step(run.app.world_mut(), run.actor, &run.targets)? {
        if rotation.driver.precombat_exhausted() {
            // SimC76f8118 player.cpp6153–6189 executes the final precombat
            // action at pull without waiting for its foreground GCD.
            if let Some(mut history) = run
                .app
                .world_mut()
                .get_mut::<crate::spell_cast::SpellHistory>(run.actor)
            {
                history.clear_fixture_precombat_gcd();
            }
            break;
        }
        super::preparation::finish_pending_setup(run)?;
    }
    run.app
        .world_mut()
        .entity_mut(run.actor)
        .remove::<crate::simc_apl_native::FixturePrecombat>();
    rotation.start_combat_at(crate::spell_cast::now(run.app.world()));
    run.gaps.retain(|gap| {
        !gap.starts_with("NATIVE_ASSISTED_COMBAT_70448_VS_SIMC_APL")
            && !gap.starts_with("NATIVE_FOLLOWER_KIT_VS_SIMC_APL")
    });
    Ok(Box::new(rotation))
}

pub(super) const REPORT_KEY: Option<&str> = Some("havoc_rotation");

impl super::SpecDriver for Rotation {
    fn label(&self) -> &'static str {
        "LITERAL_SIMC_HAVOC_76f8118"
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
