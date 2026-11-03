//! Fixture adapter over the production APL.
use bevy::prelude::*;
pub(super) use server_follower_apl::fire_rotation::*;

pub(super) fn apply_initial_spheres(world: &mut World, actor: Entity) {
    let selected = world
        .get::<crate::class_progression::KnownSpells>(actor)
        .is_some_and(|known| known.contains(448601));
    if selected {
        // WCL Gt9TAHVbDJxQd1mw fight2 pulls2/4: three earned spheres
        // persist into boss pulls. SimC76f8118 defaults to the same capped state.
        for _ in 0..3 {
            crate::spell_cast::cast_triggered(world, actor, 448604, Some(actor), 1);
        }
    }
}

pub(super) fn prepare_driver(run: &mut super::Run) -> Result<Box<dyn super::SpecDriver>, String> {
    apply_initial_spheres(run.app.world_mut(), run.actor);
    let driver = super::prepare_production_driver(run)?;
    run.gaps.retain(|gap| {
        !gap.starts_with("NATIVE_ASSISTED_COMBAT_70448_VS_SIMC_APL")
            && !gap.starts_with("NATIVE_FOLLOWER_KIT_VS_SIMC_APL")
    });
    Ok(driver)
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
