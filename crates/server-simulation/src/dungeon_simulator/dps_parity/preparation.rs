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
const MAX_SETUP_TICKS: u32 = 600;

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

pub(super) fn prepare_ret_precombat(run: &mut Run) -> Result<(), String> {
    use super::literal_rotation::PrecombatPoll;
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let mut rotation =
        super::ret_rotation::Rotation::load(run.app.world_mut(), run.actor, encounter)?;
    for _ in 0..MAX_SETUP_TICKS {
        match rotation.poll_precombat(run.app.world_mut(), run.actor, &run.targets)? {
            PrecombatPoll::Waiting => finish_pending_setup(run)?,
            PrecombatPoll::Selected { action, execution } => {
                rotation.execute_precombat(run, action, execution)?;
            }
            PrecombatPoll::Finished => {
                rotation.start_combat_at(crate::spell_cast::now(run.app.world()));
                run.ret_rotation = Some(rotation);
                return Ok(());
            }
        }
    }
    Err("native Ret precombat did not finish within the setup bound".into())
}

pub(super) fn prepare_beast_mastery_precombat(run: &mut Run) -> Result<(), String> {
    // SimC76f8118 sc_hunter.cpp1052: summon_pet defaults to duck. Literal
    // capture suppresses follower AI, so provision its existing native stable policy here.
    crate::hunter_pet::provision_follower_duck(run.app.world_mut(), run.actor)?;
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let mut rotation = super::beast_mastery_rotation::Rotation::load(
        run.app.world_mut(),
        run.actor,
        encounter,
        &run.profile,
    )?;
    while rotation.precombat_step(run.app.world_mut(), run.actor, &run.targets)? {
        finish_pending_setup(run)?;
    }
    rotation.start_combat_at(crate::spell_cast::now(run.app.world()));
    run.beast_mastery_rotation = Some(rotation);
    run.gaps
        .retain(|gap| !gap.starts_with("NATIVE_FOLLOWER_KIT_VS_SIMC_APL"));
    Ok(())
}

pub(super) fn prepare_marksmanship_precombat(run: &mut Run) -> Result<(), String> {
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let mut rotation = super::marksmanship_rotation::Rotation::load(
        run.app.world_mut(),
        run.actor,
        encounter,
        &run.profile,
    )?;
    while rotation.precombat_step(run.app.world_mut(), run.actor, &run.targets)? {
        finish_pending_setup(run)?;
    }
    rotation.start_combat_at(crate::spell_cast::now(run.app.world()));
    run.marksmanship_rotation = Some(rotation);
    Ok(())
}

pub(super) fn prepare_enhancement_precombat(run: &mut Run) -> Result<(), String> {
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let mut rotation = super::enhancement_rotation::Rotation::load(
        run.app.world_mut(),
        run.actor,
        encounter,
        &run.profile,
    )?;
    while rotation.step(run.app.world_mut(), run.actor, &run.targets, "precombat")? {
        finish_pending_setup(run)?;
    }
    rotation.start_combat_at(crate::spell_cast::now(run.app.world()));
    run.enhancement_rotation = Some(rotation);
    run.gaps.retain(|gap| {
        !gap.starts_with("NATIVE_ASSISTED_COMBAT_70448_VS_SIMC_APL")
            && !gap.starts_with("NATIVE_FOLLOWER_KIT_VS_SIMC_APL")
    });
    Ok(())
}

pub(super) fn prepare_arcane_precombat(run: &mut Run) -> Result<(), String> {
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let mut rotation = super::arcane_rotation::Rotation::load(
        run.app.world_mut(),
        run.actor,
        &run.profile,
        encounter,
    )?;
    loop {
        match rotation.precombat(run.app.world_mut(), run.actor, &run.targets)? {
            super::literal_rotation::PrecombatPoll::Waiting => finish_pending_setup(run)?,
            super::literal_rotation::PrecombatPoll::Selected { action, execution } => {
                rotation.execute(run.app.world_mut(), run.actor, action, execution)?;
                finish_pending_setup(run)?;
            }
            super::literal_rotation::PrecombatPoll::Finished => break,
        }
    }
    rotation.start_combat_at(crate::spell_cast::now(run.app.world()));
    run.arcane_rotation = Some(rotation);
    run.gaps.retain(|gap| {
        !gap.starts_with("NATIVE_ASSISTED_COMBAT_70448_VS_SIMC_APL")
            && !gap.starts_with("NATIVE_FOLLOWER_KIT_VS_SIMC_APL")
    });
    Ok(())
}

pub(super) fn prepare_elemental_precombat(run: &mut Run) -> Result<(), String> {
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let mut rotation = super::elemental_rotation::Rotation::load(
        run.app.world_mut(),
        run.actor,
        encounter,
        &run.profile,
    )?;
    while rotation.precombat_step(run.app.world_mut(), run.actor, &run.targets)? {
        finish_pending_setup(run)?;
    }
    rotation.start_combat_at(crate::spell_cast::now(run.app.world()));
    run.elemental_rotation = Some(rotation);
    run.gaps
        .retain(|gap| !gap.starts_with("NATIVE_FOLLOWER_KIT_VS_SIMC_APL"));
    Ok(())
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

pub(super) fn prepare_arms_precombat(run: &mut Run) -> Result<(), String> {
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let mut rotation = super::arms_rotation::Rotation::load(
        run.app.world_mut(),
        run.actor,
        encounter,
        &run.profile,
    )?;
    while rotation.step(run.app.world_mut(), run.actor, &run.targets, true)? {
        finish_pending_setup(run)?;
    }
    rotation.start_combat_at(crate::spell_cast::now(run.app.world()));
    run.arms_rotation = Some(rotation);
    Ok(())
}
