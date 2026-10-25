//! Read-only source deadlines and native-radius patient geometry.
use bevy::prelude::*;
use shared::components::Position;

use super::healer_triage::{Patient, TriageContext};
use super::kit::{Entry, Target};
use crate::maps::InMap;
use crate::scripted_ai::{ScheduledDamage, ScheduledDamageTarget};

/// Preparation policy, not a native spell duration or an impact forecast.
const PREPARATION_WINDOW_MS: u64 = 5_000;

#[derive(Debug)]
pub(super) struct UpcomingDamage {
    pub source: Entity,
    pub spell_id: u32,
    pub request_in_ms: u64,
    pub targets: Vec<Entity>,
}

#[derive(Debug)]
pub(super) struct InjuryCluster {
    pub spell_id: u32,
    pub anchor: Entity,
    pub members: Vec<Entity>,
}

impl TriageContext<'_> {
    pub fn preparing(&self, target: Entity) -> bool {
        self.upcoming.iter().any(|event| {
            event.request_in_ms <= PREPARATION_WINDOW_MS
                && event.targets.contains(&target)
                && self.world.get::<InMap>(event.source) == self.world.get::<InMap>(self.caster)
                && self
                    .world
                    .resource::<crate::spell_info::SpellStore>()
                    .get(event.spell_id)
                    .is_some()
        })
    }

    pub fn injury_cluster(&self, spell_id: u32, anchor: Entity) -> Option<&InjuryCluster> {
        self.clusters
            .iter()
            .find(|cluster| cluster.spell_id == spell_id && cluster.anchor == anchor)
    }
}

pub(super) fn read_upcoming(
    world: &World,
    enemies: &[(Entity, Option<Entity>)],
    patients: &[Patient],
) -> Vec<UpcomingDamage> {
    enemies
        .iter()
        .flat_map(|&(source, victim)| {
            crate::scripted_ai::read_scheduled_damage(world, source)
                .into_iter()
                .map(move |event| UpcomingDamage {
                    source,
                    spell_id: event.spell_id,
                    request_in_ms: event.request_in_ms,
                    targets: threatened_patients(world, source, victim, event, patients),
                })
        })
        .collect()
}

fn threatened_patients(
    world: &World,
    source: Entity,
    victim: Option<Entity>,
    event: ScheduledDamage,
    patients: &[Patient],
) -> Vec<Entity> {
    match event.target {
        ScheduledDamageTarget::Victim => patients
            .iter()
            .filter(|patient| Some(patient.entity) == victim)
            .map(|patient| patient.entity)
            .collect(),
        ScheduledDamageTarget::SourceArea => {
            let Some(center) = position(world, source) else {
                return Vec::new();
            };
            let Some(radius) = super::kit_area::radius_for_spell(world, source, event.spell_id)
            else {
                return Vec::new();
            };
            patients
                .iter()
                .filter(|patient| {
                    patient
                        .position
                        .is_some_and(|at| center.distance(at) <= radius)
                })
                .map(|patient| patient.entity)
                .collect()
        }
    }
}

pub(super) fn read_clusters(
    world: &World,
    caster: Entity,
    entries: &[Entry],
    patients: &[Patient],
) -> Vec<InjuryCluster> {
    entries
        .iter()
        .filter(|entry| entry.conditions.min_injured > 0)
        .flat_map(|entry| {
            patients
                .iter()
                .filter_map(move |anchor| read_cluster(world, caster, entry, anchor, patients))
        })
        .collect()
}

fn read_cluster(
    world: &World,
    caster: Entity,
    entry: &Entry,
    anchor: &Patient,
    patients: &[Patient],
) -> Option<InjuryCluster> {
    let radius = super::kit_area::spell_radius(world, caster, entry)?;
    let source_centered =
        entry.conditions.source_center || entry.conditions.target == Target::SelfUnit;
    let center = if source_centered {
        position(world, caster)?
    } else {
        anchor.position?
    };
    let members = patients
        .iter()
        .filter(|patient| {
            patient.missing_health > 0.0
                && patient
                    .position
                    .is_some_and(|at| center.distance(at) <= radius)
        })
        .map(|patient| patient.entity)
        .collect();
    Some(InjuryCluster {
        spell_id: entry.spell_id,
        anchor: anchor.entity,
        members,
    })
}

pub(super) fn position(world: &World, target: Entity) -> Option<Vec3> {
    world
        .get::<Position>(target)
        .map(|at| Vec3::new(at.x, at.y, at.z))
}
