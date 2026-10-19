//! Run-local observations from ordinary combat events and follower fixed updates.
use bevy::prelude::*;
use shared::components::Health;
use shared::protocol::{CombatLogEvent, CombatLogKind};
use std::collections::{BTreeSet, VecDeque};

use crate::maps::InMap;

const RATE_SECONDS: f64 = 10.0;
const HEALTH_SECONDS: f64 = 5.0;
const MIN_HEALTH_FRACTION: f32 = 0.80;

#[derive(Debug)]
pub(super) struct Snapshot {
    pub observation_seconds: f64,
    pub group_dps: Option<f64>,
    pub healer_hps: Option<f64>,
    pub completed_fights: u32,
    pub party_stable: bool,
    pub cleared_spawns: BTreeSet<u32>,
}

#[derive(Clone, Copy)]
struct HealthSample {
    at: f64,
    healthy: bool,
}

struct OutputSample {
    at: f64,
    damage: f64,
    healing: f64,
}

#[derive(Component)]
struct PullHistory {
    map: InMap,
    members: Vec<Entity>,
    started: f64,
    health: VecDeque<HealthSample>,
    output: VecDeque<OutputSample>,
    active_enemies: BTreeSet<Entity>,
    cleared_spawns: BTreeSet<u32>,
    completed_fights: u32,
}

impl PullHistory {
    fn new(map: InMap, members: Vec<Entity>, now: f64) -> Self {
        Self {
            map,
            members,
            started: now,
            health: VecDeque::new(),
            output: VecDeque::new(),
            active_enemies: BTreeSet::new(),
            cleared_spawns: BTreeSet::new(),
            completed_fights: 0,
        }
    }

    fn observe_health(&mut self, now: f64, healthy: bool) {
        self.health.push_back(HealthSample { at: now, healthy });
        // Retain the sample covering the left boundary, not an assumed healthy gap.
        while self.health.len() > 1 && self.health[1].at <= now - HEALTH_SECONDS {
            self.health.pop_front();
        }
        while self
            .output
            .front()
            .is_some_and(|sample| sample.at < now - RATE_SECONDS)
        {
            self.output.pop_front();
        }
    }

    fn party_stable(&self, now: f64) -> bool {
        let complete = self
            .health
            .front()
            .is_some_and(|sample| sample.at <= now - HEALTH_SECONDS);
        let current = self.health.back().is_some_and(|sample| sample.at == now);
        complete && current && self.health.iter().all(|sample| sample.healthy)
    }

    fn rates(&self, now: f64) -> Option<(f64, f64)> {
        if now - self.started < RATE_SECONDS {
            return None;
        }
        let recent = self
            .output
            .iter()
            .filter(|sample| sample.at >= now - RATE_SECONDS);
        let (damage, healing) = recent.fold((0.0, 0.0), |(damage, healing), sample| {
            (damage + sample.damage, healing + sample.healing)
        });
        Some((damage / RATE_SECONDS, healing / RATE_SECONDS))
    }

    fn observe_death(&mut self, target: Entity, guid: Option<u32>) {
        if let Some(guid) = guid.filter(|guid| *guid != 0) {
            self.cleared_spawns.insert(guid);
        }
        if self.active_enemies.remove(&target) && self.active_enemies.is_empty() {
            self.completed_fights += 1;
        }
    }
}

fn read_time(world: &World) -> f64 {
    world.resource::<Time>().elapsed_secs_f64()
}

fn party_healthy(world: &World, members: &[Entity], map: InMap) -> bool {
    members.iter().all(|&member| {
        world.get::<InMap>(member) == Some(&map)
            && world.get::<Health>(member).is_some_and(|health| {
                health.current.is_finite()
                    && health.max.is_finite()
                    && health.max > 0.0
                    && health.current >= health.max * MIN_HEALTH_FRACTION
            })
    })
}

pub(super) fn sample_all(world: &mut World) {
    let mut owners: Vec<_> = world
        .query::<&super::Follower>()
        .iter(world)
        .map(|follower| follower.owner)
        .collect();
    owners.sort_by_key(|owner| owner.to_bits());
    owners.dedup();
    for owner in owners {
        sample_party(world, owner);
    }
}

fn read_current_party(world: &World, owner: Entity) -> Option<(InMap, Vec<Entity>)> {
    let (map, admitted, _) = super::party::read_admission(world, owner)?;
    let mut members = admitted.to_vec();
    members.push(owner);
    members.sort_by_key(|member| member.to_bits());
    Some((map, members))
}

fn sample_party(world: &mut World, owner: Entity) {
    let Some((map, members)) = read_current_party(world, owner) else {
        if let Ok(mut owner) = world.get_entity_mut(owner) {
            owner.remove::<PullHistory>();
        }
        return;
    };
    let now = read_time(world);
    let healthy = party_healthy(world, &members, map);
    let same_run = world
        .get::<PullHistory>(owner)
        .is_some_and(|history| history.map == map && history.members == members);
    if !same_run {
        world
            .entity_mut(owner)
            .insert(PullHistory::new(map, members, now));
    }
    world
        .get_mut::<PullHistory>(owner)
        .unwrap()
        .observe_health(now, healthy);
}

pub(super) fn snapshot(world: &World, owner: Entity) -> Option<Snapshot> {
    let history = world.get::<PullHistory>(owner)?;
    let (map, members) = read_current_party(world, owner)?;
    if history.map != map || history.members != members {
        return None;
    }
    let now = read_time(world);
    let rates = history.rates(now);
    Some(Snapshot {
        observation_seconds: now - history.started,
        group_dps: rates.map(|rates| rates.0),
        healer_hps: rates.map(|rates| rates.1),
        completed_fights: history.completed_fights,
        party_stable: history.party_stable(now),
        cleared_spawns: history.cleared_spawns.clone(),
    })
}

fn party_source(world: &World, source: Entity) -> Entity {
    if let Some(summon) = world.get::<crate::spell_cast::summon::SummonedCreature>(source) {
        return party_source(world, summon.owner);
    }
    crate::spell_cast::combat_event_owner(world, source)
}

struct EventObservation {
    owner: Entity,
    source_member: bool,
    target_member: bool,
    healthy: bool,
}

struct EventInput<'a> {
    event: &'a CombatLogEvent,
    source: Option<Entity>,
    target: Entity,
    guid: Option<u32>,
    now: f64,
}

fn read_event_parties(
    world: &mut World,
    map: InMap,
    source: Option<Entity>,
    target: Entity,
) -> Vec<EventObservation> {
    world
        .query::<(Entity, &PullHistory)>()
        .iter(world)
        .filter(|(_, history)| history.map == map)
        .map(|(owner, history)| EventObservation {
            owner,
            source_member: source.is_some_and(|source| history.members.contains(&source)),
            target_member: history.members.contains(&target),
            healthy: party_healthy(world, &history.members, map),
        })
        .collect()
}

/// The ordinary log boundary observes damage/healing after native mitigation/clamping,
/// and deaths before corpse cleanup; disappearance never enters this path as death.
pub(crate) fn record_event(world: &mut World, event: &CombatLogEvent) {
    if !matches!(
        event.kind,
        CombatLogKind::Damage | CombatLogKind::Heal | CombatLogKind::Death
    ) {
        return;
    }
    let Some(target) = event.target.and_then(Entity::try_from_bits) else {
        return;
    };
    let Some(map) = world.get::<InMap>(target).copied() else {
        return;
    };
    let source = event.source.and_then(Entity::try_from_bits);
    let credited = source
        .filter(|source| world.get::<InMap>(*source) == Some(&map))
        .map(|source| party_source(world, source));
    let guid = world
        .get::<crate::zones::CreatureOrigin>(target)
        .map(|origin| origin.guid);
    let observations = read_event_parties(world, map, credited, target);
    if observations.is_empty() {
        return;
    }
    let input = EventInput {
        event,
        source,
        target,
        guid,
        now: read_time(world),
    };
    for observation in observations {
        let mut history = world.get_mut::<PullHistory>(observation.owner).unwrap();
        observe_event(&mut history, &input, &observation);
    }
}

fn observe_event(
    history: &mut PullHistory,
    input: &EventInput<'_>,
    observation: &EventObservation,
) {
    if observation.target_member {
        history.observe_health(input.now, observation.healthy);
    }
    match input.event.kind {
        CombatLogKind::Damage => observe_damage(history, input, observation),
        CombatLogKind::Heal if observation.source_member && observation.target_member => {
            history.output.push_back(OutputSample {
                at: input.now,
                damage: 0.0,
                healing: effective(input.event),
            });
        }
        CombatLogKind::Death if !observation.target_member => {
            history.observe_death(input.target, input.guid)
        }
        _ => {}
    }
}

fn effective(event: &CombatLogEvent) -> f64 {
    f64::from(event.amount.saturating_sub(event.overflow.max(0)).max(0))
}

fn observe_damage(
    history: &mut PullHistory,
    input: &EventInput<'_>,
    observation: &EventObservation,
) {
    if observation.source_member && !observation.target_member {
        history.active_enemies.insert(input.target);
        history.output.push_back(OutputSample {
            at: input.now,
            damage: effective(input.event),
            healing: 0.0,
        });
    } else if observation.target_member && !observation.source_member {
        if let Some(source) = input.source {
            history.active_enemies.insert(source);
        }
    }
}
