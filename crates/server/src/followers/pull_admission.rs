//! Native composition of a projected source footprint with the current run's budget.
use bevy::prelude::*;
use shared::components::Health;
use shared::group::GroupRole;
use std::collections::{BTreeMap, BTreeSet};

use super::pull_budget::{self, BudgetInput, Decision, Reason};
use super::pull_history::Snapshot;
use super::pull_sources::Source;
use super::{Follower, ai, party, pull_history, pull_readiness, pull_sources};
use crate::combat::forecast::{self, MeleeEstimate};
use crate::maps::InMap;

struct RunBudget {
    follower: Follower,
    map: InMap,
    party: Vec<Entity>,
    history: Snapshot,
}

struct Projection {
    estimates: Vec<(u32, MeleeEstimate)>,
    pack_count: usize,
    overlapping: bool,
}

pub(super) fn read_pull_budget(
    world: &mut World,
    tank: Entity,
    projected: &BTreeSet<u32>,
    difficulty: u32,
) -> Result<Decision, String> {
    let run = read_run_budget(world, tank)?;
    if let Some(decision) = read_cleared_single_pull(world, tank, &run, projected, difficulty)? {
        return Ok(decision);
    }
    let Some(ready) = pull_readiness::read_ready_heal(world, tank, &run.party)? else {
        return Ok(Decision::Wait(Reason::Healer));
    };
    debug!(healer=?ready.healer, spell_id=ready.spell_id, costs=?ready.costs, "Sourced tank healing capability checked");
    let projection = read_projection(world, tank, &run, projected, difficulty)?;
    let input = read_budget_input(world, tank, &run, &projection)?;
    Ok(pull_budget::evaluate(&input))
}

/// User policies apply only to a cleared copy and one next authored pack.
/// No stat snapshot or invented numeric forecast can veto this ready single pull.
fn read_cleared_single_pull(
    world: &mut World,
    tank: Entity,
    run: &RunBudget,
    projected: &BTreeSet<u32>,
    difficulty: u32,
) -> Result<Option<Decision>, String> {
    let fighting = pull_history::has_attackable_combat(world, &run.party);
    if fighting || !ai::engaged_enemies(world, &run.follower, tank, &run.party).is_empty() {
        return Ok(None);
    }
    if !run.history.active_enemies.is_empty() {
        // Lost detection/combat flags are not observed Death receipts.
        return Ok(Some(Decision::Wait(Reason::Engaged)));
    }
    if projected.is_empty() {
        return Err("A new pull requires a nonempty projected source footprint".into());
    }
    let sources = pull_sources::read_present_sources(
        world,
        run.map,
        difficulty,
        &run.history.cleared_spawns,
    )?;
    let packs: BTreeSet<_> = projected
        .iter()
        .map(|guid| read_source_pack(&sources, *guid))
        .collect::<Result<_, _>>()?;
    if packs.len() != 1 {
        return Ok(Some(Decision::Wait(Reason::FirstSinglePack)));
    }
    if let Some(reason) = read_party_recovery(world, run)? {
        return Ok(Some(Decision::Wait(reason)));
    }
    if !has_admitted_healer(world, run) {
        return Ok(Some(Decision::Wait(Reason::Healer)));
    }
    let targets: Vec<_> = projected
        .iter()
        .map(|guid| {
            sources[guid]
                .actor
                .ok_or_else(|| format!("Ready pull source GUID {guid} lacks its native actor"))
        })
        .collect::<Result<_, _>>()?;
    if targets
        .iter()
        .any(|target| super::hazards::target_is_hazardous(world, tank, *target))
    {
        return Ok(Some(Decision::Wait(Reason::Hazard)));
    }
    let scripted_boss = targets.iter().any(|target| {
        world
            .get::<crate::scripted_ai::CreatureScriptName>(*target)
            .is_some_and(|name| name.0.starts_with("boss_"))
    });
    let policy = if scripted_boss {
        "SCRIPTED_BOSS_PULL_WHEN_READY"
    } else {
        "CLEARED_STATE_ALWAYS_PULLS_NEXT_PACK"
    };
    debug!(
        policy,
        ?tank,
        ?packs,
        "Ready single-pack pull without numerical forecast"
    );
    // Full positive tank HP preserves the 30% reserve without a fake forecast.
    Ok(Some(Decision::Allow {
        bootstrap: run.history.completed_fights == 0,
        forecast_health: None,
    }))
}

fn read_party_recovery(world: &World, run: &RunBudget) -> Result<Option<Reason>, String> {
    for &unit in &run.party {
        if world.get::<InMap>(unit) != Some(&run.map) {
            return Ok(Some(Reason::Recovery));
        }
        let Some(health) = world.get::<Health>(unit) else {
            return Ok(Some(Reason::DeadMember));
        };
        if health.current <= 0.0 {
            return Ok(Some(Reason::DeadMember));
        }
        let mana = world
            .get::<shared::components::Mana>(unit)
            .ok_or_else(|| format!("Ready pull member {unit:?} lacks native mana state"))?;
        let valid = health.current.is_finite()
            && health.max.is_finite()
            && health.max > 0.0
            && mana.current.is_finite()
            && mana.max.is_finite()
            && mana.max >= 0.0;
        if !valid {
            return Err(format!(
                "Ready pull member {unit:?} has invalid native HP/mana"
            ));
        }
        if health.current < health.max || mana.current < mana.max {
            return Ok(Some(Reason::Recovery));
        }
    }
    Ok(None)
}

fn has_admitted_healer(world: &World, run: &RunBudget) -> bool {
    let owner_heals = party::read_admission(world, run.follower.owner)
        .is_some_and(|(_, _, role)| role == GroupRole::Healer);
    owner_heals
        || run.party.iter().any(|unit| {
            world
                .get::<Follower>(*unit)
                .is_some_and(|follower| follower.profile.role == GroupRole::Healer)
        })
}

/// Native numeric report uses exactly the input and arithmetic used for overlap admission.
#[cfg(test)]
pub(super) fn read_forecast_health(
    world: &mut World,
    tank: Entity,
    projected: &BTreeSet<u32>,
    difficulty: u32,
) -> Result<(f64, f64, usize), String> {
    let run = read_run_budget(world, tank)?;
    let projection = read_projection(world, tank, &run, projected, difficulty)?;
    let input = read_budget_input(world, tank, &run, &projection)?;
    pull_budget::read_forecast_health(&input)
        .map(|(forecast, reserve)| (forecast, reserve, projection.pack_count))
        .ok_or("Native forecast report lacks valid measured inputs".into())
}

fn read_run_budget(world: &World, tank: Entity) -> Result<RunBudget, String> {
    let follower = world
        .get::<Follower>(tank)
        .cloned()
        .ok_or("Pull budget requires a follower tank")?;
    if follower.profile.role != GroupRole::Tank {
        return Err("Pull budget requires the admitted tank role".into());
    }
    let (map, members, _) = party::read_admission(world, follower.owner)
        .ok_or("Pull budget requires one authoritative admitted party")?;
    if !members.contains(&tank) || world.get::<InMap>(tank) != Some(&map) {
        return Err("Tank is not a member of the admitted copy".into());
    }
    let mut party = members.to_vec();
    party.push(follower.owner);
    let history = pull_history::snapshot(world, follower.owner)
        .ok_or("Pull budget lacks current-run observation history")?;
    Ok(RunBudget {
        follower,
        map,
        party,
        history,
    })
}

fn read_projection(
    world: &mut World,
    tank: Entity,
    run: &RunBudget,
    projected: &BTreeSet<u32>,
    difficulty: u32,
) -> Result<Projection, String> {
    if projected.is_empty() {
        return Err("A new pull requires a nonempty projected source footprint".into());
    }
    let sources = pull_sources::read_present_sources(
        world,
        run.map,
        difficulty,
        &run.history.cleared_spawns,
    )?;
    let engaged: BTreeSet<_> = ai::engaged_enemies(world, &run.follower, tank, &run.party)
        .into_iter()
        .map(|(unit, _)| unit)
        .chain(run.history.active_enemies.iter().copied())
        .collect();
    let engaged_packs = read_engaged_packs(world, &sources, &engaged)?;
    let mut packs: BTreeSet<_> = projected
        .iter()
        .map(|guid| read_source_pack(&sources, *guid))
        .collect::<Result<_, _>>()?;
    let overlapping =
        !engaged_packs.is_empty() && packs.iter().any(|pack| !engaged_packs.contains(pack));
    packs.extend(engaged_packs);
    let estimates = sources
        .values()
        .filter(|source| packs.contains(&source.pack_guid))
        .map(|source| read_source_forecast(world, tank, source))
        .collect::<Result<_, _>>()?;
    Ok(Projection {
        estimates,
        pack_count: packs.len(),
        overlapping,
    })
}

fn read_source_pack(sources: &BTreeMap<u32, Source>, guid: u32) -> Result<u32, String> {
    sources
        .get(&guid)
        .map(|source| source.pack_guid)
        .ok_or_else(|| format!("Projected source GUID {guid} is not present in this run"))
}

fn read_engaged_packs(
    world: &World,
    sources: &BTreeMap<u32, Source>,
    engaged: &BTreeSet<Entity>,
) -> Result<BTreeSet<u32>, String> {
    engaged
        .iter()
        .map(|unit| {
            let origin = world
                .get::<crate::zones::CreatureOrigin>(*unit)
                .ok_or_else(|| {
                    format!("Engaged actor {unit:?} has no source GUID for forecasting")
                })?;
            read_source_pack(sources, origin.guid)
        })
        .collect()
}

fn read_source_forecast(
    world: &World,
    tank: Entity,
    source: &Source,
) -> Result<(u32, MeleeEstimate), String> {
    let actor = source.actor.ok_or_else(|| {
        format!(
            "Source GUID {} remains present but has no forecast actor",
            source.guid
        )
    })?;
    let estimate = forecast::read_generic_creature(world, actor, tank).map_err(|error| {
        format!(
            "Source GUID {} entry {}: {error}",
            source.guid, source.entry
        )
    })?;
    Ok((source.guid, estimate))
}

fn read_budget_input(
    world: &World,
    tank: Entity,
    run: &RunBudget,
    projection: &Projection,
) -> Result<BudgetInput, String> {
    let health = world
        .get::<Health>(tank)
        .ok_or("Pull budget lacks native tank health")?;
    let hostile_health = projection
        .estimates
        .iter()
        .map(|(_, estimate)| estimate.remaining_health)
        .sum();
    let (incoming_dps, opening_burst) =
        read_damage_debt(world, &run.history, projection, hostile_health)?;
    Ok(BudgetInput {
        observation_seconds: run.history.observation_seconds,
        completed_fights: run.history.completed_fights,
        pack_count: projection.pack_count,
        overlapping: projection.overlapping,
        party_stable: run.history.party_stable,
        healer_available: true,
        tank_health: f64::from(health.current),
        tank_max_health: f64::from(health.max),
        hostile_health,
        group_dps: run.history.group_dps,
        healer_hps: run.history.healer_hps,
        incoming_dps,
        opening_burst,
        confirmed_cooldown_credit: 0.0,
    })
}

fn read_damage_debt(
    world: &World,
    history: &Snapshot,
    projection: &Projection,
    health: f64,
) -> Result<(f64, f64), String> {
    // These policy branches never produce a numeric forecast; do not invent a TTK.
    let Some(duration) = read_clearance_duration(history, health)? else {
        let steady = projection
            .estimates
            .iter()
            .map(|(_, estimate)| estimate.expected_hit / estimate.interval_secs)
            .sum();
        return Ok((steady, 0.0));
    };
    let now = crate::auras::aura_now(world);
    let mut later = 0.0;
    let mut opening = 0.0;
    for (guid, estimate) in &projection.estimates {
        if estimate
            .valid_until
            .is_some_and(|until| now + duration > until)
        {
            return Err(format!(
                "Source GUID {guid} level{} rank{} stat snapshot expires before measured clearance",
                estimate.level, estimate.rank
            ));
        }
        let (remaining, first) = swing_damage_debt(estimate, duration);
        later += remaining;
        opening += first;
    }
    Ok((later / duration, opening))
}

fn read_clearance_duration(history: &Snapshot, health: f64) -> Result<Option<f64>, String> {
    if history.completed_fights == 0 {
        return Ok(None);
    }
    let Some(damage) = history
        .group_dps
        .filter(|value| value.is_finite() && *value > 0.0)
    else {
        return Ok(None);
    };
    let duration = health / damage;
    if !duration.is_finite() || duration <= 0.0 {
        return Err("Measured source clearance duration is invalid".into());
    }
    Ok(Some(duration))
}

fn swing_damage_debt(estimate: &MeleeEstimate, duration: f64) -> (f64, f64) {
    if duration < estimate.next_swing_secs {
        return (0.0, 0.0);
    }
    let later_swings = ((duration - estimate.next_swing_secs) / estimate.interval_secs).floor();
    // The first native hit is counted only in the opening, never again in sustained debt.
    (later_swings * estimate.expected_hit, estimate.largest_hit)
}
