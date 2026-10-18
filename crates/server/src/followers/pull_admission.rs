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
    let Some(ready) = pull_readiness::read_ready_heal(world, tank, &run.party)? else {
        return Ok(Decision::Wait(Reason::Healer));
    };
    debug!(healer=?ready.healer, spell_id=ready.spell_id, costs=?ready.costs, "Sourced tank healing capability checked");
    let projection = read_projection(world, tank, &run, projected, difficulty)?;
    let input = read_budget_input(world, tank, &run, &projection)?;
    Ok(pull_budget::evaluate(&input))
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
    let engaged = ai::engaged_enemies(world, &run.follower, tank, &run.party);
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
    engaged: &[(Entity, Option<Entity>)],
) -> Result<BTreeSet<u32>, String> {
    engaged
        .iter()
        .map(|(unit, _)| {
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
