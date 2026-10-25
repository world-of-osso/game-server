//! Native setup advances real cast/GCD state before fixture engagement.
use super::Run;

/// Complete selected native work before another source row or fixture engagement.
pub(super) fn execute_precombat_plan(
    run: &mut Run,
    driver: &mut super::literal_rotation::Driver,
    action: server::simc_apl::Action,
    execution: super::literal_rotation::execution::Execution,
) -> Result<(), String> {
    use super::literal_rotation::execution::Execution;
    let steps = match execution {
        Execution::Composite(steps) if !steps.is_empty() => steps,
        Execution::Composite(_) => return Err("selected native composite plan is empty".into()),
        execution => vec![execution],
    };
    for step in steps {
        // Every dispatch revalidates native admission; only the real App completes it.
        driver.execute(run.app.world_mut(), run.actor, action.clone(), step)?;
        finish_pending_setup(run)?;
    }
    Ok(())
}

// Bound setup failures; advancing frames must not become an endless cast retry.
pub(super) const MAX_SETUP_TICKS: u32 = 600;

pub(super) fn start_configured_native_pet(run: &mut Run) -> Result<(), String> {
    let configured = super::literal_rotation::default_pet::ConfiguredPet::read(
        run.app.world(),
        run.actor,
        &run.profile,
    )?;
    let Some(configured) = configured else {
        return Ok(());
    };
    let Some(action) = wait_for_configured_pet_action(run, &configured)? else {
        return Ok(());
    };
    super::literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        super::literal_rotation::execution::Execution::Spell(action.spell, run.actor, None),
    )
}

pub(super) fn wait_for_configured_pet_action(
    run: &mut Run,
    configured: &super::literal_rotation::default_pet::ConfiguredPet,
) -> Result<Option<super::literal_rotation::execution::NativeAction<'static>>, String> {
    for _ in 0..MAX_SETUP_TICKS {
        let Some(action) = configured.read_action(run.app.world(), run.actor)? else {
            return Ok(None);
        };
        if read_precast_readiness(run, action.spell)? {
            return Ok(Some(action));
        }
        run.app.update();
    }
    Err(format!(
        "configured native summon not affordable/ready after30s OOC: {}",
        super::report::engagement(run.app.world(), run.actor, &run.targets)
    ))
}

fn read_precast_readiness(run: &mut Run, id: u32) -> Result<bool, String> {
    use crate::spell_cast::CastFailure;
    let spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(id)
        .cloned()
        .ok_or("configured summon missing from native SpellStore")?;
    match crate::spell_cast::read_native_cast_start(
        run.app.world_mut(),
        run.actor,
        &spell,
        run.actor,
    ) {
        Ok(_) => Ok(true),
        Err(
            CastFailure::NotEnoughPower(_)
            | CastFailure::InProgress
            | CastFailure::OnGlobalCooldown,
        ) => Ok(false),
        Err(error) => Err(format!("configured native summon{id} rejected: {error:?}")),
    }
}

pub(super) fn consume_profile_food(run: &mut Run) -> Result<(), String> {
    // SimC76f8118 unique_gear_midnight.cpp85–103 selects composed ratings at food start.
    // Both literal and ordinary setup must finish native gear/passive initialization first.
    warm_up_reference(run)?;
    super::profile_effects::install_food(run.app.world_mut(), run.actor, &run.profile)?;
    finish_pending_setup(run)
}

pub(super) fn warm_up_reference(run: &mut Run) -> Result<(), String> {
    for _ in 0..4 {
        run.app.update();
    }
    finish_pending_setup(run)
}

pub(super) fn finish_pending_setup(run: &mut Run) -> Result<(), String> {
    // Instant NPC casts have no GCD, but their queued native effects still need a frame.
    run.app.update();
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
