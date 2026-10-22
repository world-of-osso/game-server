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
    pub active_enemies: BTreeSet<Entity>,
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
    combat_seconds: f64,
    last_observation: f64,
    in_combat: bool,
    wiped: bool,
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
            combat_seconds: 0.0,
            last_observation: now,
            in_combat: false,
            wiped: false,
            health: VecDeque::new(),
            output: VecDeque::new(),
            active_enemies: BTreeSet::new(),
            cleared_spawns: BTreeSet::new(),
            completed_fights: 0,
        }
    }

    fn observe_clock(&mut self, now: f64, in_combat: bool) {
        if self.in_combat {
            self.combat_seconds += (now - self.last_observation).max(0.0);
        }
        self.last_observation = now;
        self.in_combat = in_combat;
        while self
            .output
            .front()
            .is_some_and(|sample| sample.at < self.combat_seconds - RATE_SECONDS)
        {
            self.output.pop_front();
        }
    }

    fn reset_measurements(&mut self, now: f64) {
        self.combat_seconds = 0.0;
        self.last_observation = now;
        self.in_combat = false;
        self.health.clear();
        self.output.clear();
        self.active_enemies.clear();
        self.completed_fights = 0;
    }

    fn observe_health(&mut self, now: f64, healthy: bool) {
        self.health.push_back(HealthSample { at: now, healthy });
        // Retain the sample covering the left boundary, not an assumed healthy gap.
        while self.health.len() > 1 && self.health[1].at <= now - HEALTH_SECONDS {
            self.health.pop_front();
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

    fn rates(&self) -> Option<(f64, f64)> {
        if self.combat_seconds < RATE_SECONDS {
            return None;
        }
        let recent = self
            .output
            .iter()
            .filter(|sample| sample.at >= self.combat_seconds - RATE_SECONDS);
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
            self.in_combat = false;
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
    let in_combat = members.iter().any(|member| {
        world
            .get::<shared::components::CombatStatus>(*member)
            .is_some_and(|status| status.0)
    });
    let wiped = members.iter().all(|member| {
        world
            .get::<Health>(*member)
            .is_some_and(|health| health.current <= 0.0)
    });
    let same_run = world
        .get::<PullHistory>(owner)
        .is_some_and(|history| history.map == map && history.members == members);
    if !same_run {
        world
            .entity_mut(owner)
            .insert(PullHistory::new(map, members, now));
    }
    let mut history = world.get_mut::<PullHistory>(owner).unwrap();
    history.observe_clock(now, in_combat && !wiped);
    if wiped && !history.wiped {
        history.reset_measurements(now);
    }
    history.wiped = wiped;
    history.observe_health(now, healthy);
}

fn current_history(world: &World, owner: Entity) -> Option<&PullHistory> {
    let history = world.get::<PullHistory>(owner)?;
    let (map, members) = read_current_party(world, owner)?;
    (history.map == map && history.members == members).then_some(history)
}

pub(super) fn has_uncleared_fight(world: &World, owner: Entity) -> bool {
    current_history(world, owner).is_some_and(|history| !history.active_enemies.is_empty())
}

pub(super) fn snapshot(world: &World, owner: Entity) -> Option<Snapshot> {
    let history = current_history(world, owner)?;
    let now = read_time(world);
    let rates = history.rates();
    Some(Snapshot {
        observation_seconds: history.combat_seconds,
        group_dps: rates.map(|rates| rates.0),
        healer_hps: rates.map(|rates| rates.1),
        completed_fights: history.completed_fights,
        party_stable: history.party_stable(now),
        cleared_spawns: history.cleared_spawns.clone(),
        active_enemies: history.active_enemies.clone(),
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
    in_combat: bool,
}

struct EventInput<'a> {
    event: &'a CombatLogEvent,
    living_source: Option<Entity>,
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
            in_combat: history.members.iter().any(|member| {
                world
                    .get::<shared::components::CombatStatus>(*member)
                    .is_some_and(|status| status.0)
            }),
        })
        .collect()
}

/// The ordinary log boundary observes damage/healing after native mitigation/clamping,
/// and deaths before corpse cleanup; disappearance never enters this path as death.
pub(crate) fn record_event(world: &mut World, event: &CombatLogEvent) {
    super::healer_triage::record_damage(world, event);
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
        living_source: source.filter(|source| crate::spell_cast::targets::is_alive(world, *source)),
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
    history.observe_clock(input.now, observation.in_combat);
    if observation.target_member {
        history.observe_health(input.now, observation.healthy);
    }
    match input.event.kind {
        CombatLogKind::Damage => observe_damage(history, input, observation),
        CombatLogKind::Heal
            if observation.source_member && observation.target_member && history.in_combat =>
        {
            history.output.push_back(OutputSample {
                at: history.combat_seconds,
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
        history.in_combat = true;
        history.output.push_back(OutputSample {
            at: history.combat_seconds,
            damage: effective(input.event),
            healing: 0.0,
        });
    } else if observation.target_member && !observation.source_member {
        // Lingering periodic damage remains real after its caster dies, but a
        // dead source cannot reopen the encounter already closed by Death.
        if let Some(source) = input.living_source {
            history.active_enemies.insert(source);
            history.in_combat = true;
        }
    }
}
