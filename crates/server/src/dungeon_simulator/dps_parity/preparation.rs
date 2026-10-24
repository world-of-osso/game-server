//! Native setup advances real cast/GCD state before fixture engagement.
use super::Run;

// Bound setup failures; advancing frames must not become an endless cast retry.
const MAX_SETUP_TICKS: u32 = 600;

pub(super) fn warm_up_reference(run: &mut Run) -> Result<(), String> {
    for _ in 0..4 {
        run.app.update();
    }
    finish_pending_setup(run)
}

pub(super) fn finish_pending_setup(run: &mut Run) -> Result<(), String> {
    for _ in 0..MAX_SETUP_TICKS {
        if !setup_cast_or_gcd_active(run) {
            return Ok(());
        }
        run.app.update();
    }
    Err(format!(
        "native fixture setup exceeded30s before engagement; actor state {}",
        super::report::engagement(run.app.world(), run.actor, &run.targets)
    ))
}

fn setup_cast_or_gcd_active(run: &Run) -> bool {
    let world = run.app.world();
    world.get::<shared::casting::CastState>(run.actor).is_some()
        || world
            .get::<crate::spell_cast::SpellHistory>(run.actor)
            .is_some_and(|history| history.has_active_gcd(crate::spell_cast::history_now(world)))
}

pub(super) fn prepare_windwalker_precombat(run: &mut Run) -> Result<(), String> {
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let mut rotation =
        super::windwalker_rotation::Rotation::load(run.app.world_mut(), run.actor, encounter)?;
    while rotation.precombat_step(run.app.world_mut(), run.actor, &run.targets)? {
        finish_pending_setup(run)?;
    }
    rotation.start_combat_at(crate::spell_cast::now(run.app.world()));
    run.windwalker_rotation = Some(rotation);
    Ok(())
}
