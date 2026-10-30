//! Reproduce the actual pinned SimC precombat actions before engaging a follower.
use crate::simc_tank_timeline;
use bevy::prelude::*;

pub(super) fn execute(
    app: &mut App,
    actor: Entity,
    target: Entity,
    trace_path: &std::path::Path,
) -> Result<(), String> {
    let scenario = simc_tank_timeline::read_scenario(trace_path)?;
    let trace = std::fs::read_to_string(trace_path).map_err(|error| error.to_string())?;
    let end = format!("Combat begins for Player '{}'.", scenario.player);
    let precombat = trace
        .split_once(&end)
        .ok_or("SIMC_PRECOMBAT_BOUNDARY_ABSENT")?
        .0;
    let marker = format!("Player '{}' performs Action '", scenario.player);
    for line in precombat.lines() {
        let Some((_, action)) = line.split_once(&marker) else {
            continue;
        };
        let (name, rest) = action
            .split_once("' (")
            .ok_or("SIMC_PRECOMBAT_ACTION_INVALID")?;
        let id = rest
            .split_once(')')
            .ok_or("SIMC_PRECOMBAT_SPELL_INVALID")?
            .0
            .parse::<u32>()
            .map_err(|error| error.to_string())?;
        if id == 0 {
            return Err(format!("SIMC_PRECOMBAT_NON_SPELL_UNSUPPORTED:{name}"));
        }
        let chosen = match crate::spell_cast::creature::fixture_ai_spell_target(app.world(), id) {
            crate::spell_cast::creature::AiTarget::SelfTarget => actor,
            _ => target,
        };
        crate::spell_cast::creature_cast(app.world_mut(), actor, id, Some(chosen), false)
            .map_err(|error| format!("SIMC_PRECOMBAT_CAST_FAILED:{name}:{id}:{error:?}"))?;
        if app
            .world()
            .get::<shared::casting::CastState>(actor)
            .is_some()
        {
            return Err(format!("SIMC_PRECOMBAT_CAST_TIME_UNSUPPORTED:{name}:{id}"));
        }
        if id == 26573 {
            let marker = "used Consecration in precombat with precombat time = ";
            let seconds = precombat
                .lines()
                .find_map(|line| line.split_once(marker).map(|(_, value)| value))
                .and_then(|value| value.split(',').next())
                .ok_or("SIMC_PRECOMBAT_CONSECRATION_TIME_ABSENT")?
                .parse::<f64>()
                .map_err(|error| error.to_string())?;
            if !seconds.is_finite() || seconds < 0.0 {
                return Err("SIMC_PRECOMBAT_TIME_INVALID".into());
            }
            crate::spell_cast::scripts::prepare_simc_precombat_consecration(
                app.world_mut(),
                actor,
                seconds,
            )?;
        }
        if let Some(mut history) = app
            .world_mut()
            .get_mut::<crate::spell_cast::SpellHistory>(actor)
        {
            history.clear_fixture_precombat_gcd();
        }
    }
    Ok(())
}
