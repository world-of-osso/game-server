//! Stable, pre-target-mitigation receipts. Never reconstructed from combat-log loss.
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) enum HitKind {
    Melee {
        hand: u8,
        outcome: String,
        level_difference: i32,
        critical_multiplier: f32,
        weapon_speed: f32,
    },
    Spell {
        spell_id: u32,
        depth: u8,
        bonus_source_guid: Option<u32>,
        effect_index: Option<u32>,
        periodic: bool,
        crit: bool,
        resolved_source: bool,
        blocked: bool,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) struct Hit {
    pub tick: u32,
    pub source_guid: u32,
    pub pack: u32,
    pub target_slot: usize,
    pub school: u32,
    /// Source bonuses/crit already applied; target reductions/armor/absorbs not applied.
    pub raw: f32,
    pub kind: HitKind,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(super) struct SourceSnapshot {
    pub entry: u32,
    pub native_level: u8,
    pub scaling: Option<shared::level_scaling::LevelScaling>,
}

pub(super) fn snapshot_sources(
    world: &World,
    actors: &BTreeMap<u32, Entity>,
) -> Result<BTreeMap<u32, SourceSnapshot>, String> {
    actors
        .iter()
        .map(|(&guid, &unit)| {
            let entry = world
                .get::<shared::components::Npc>(unit)
                .ok_or_else(|| format!("SOURCE_NPC_IDENTITY_ABSENT:{guid}"))?
                .template_id;
            let native_level = world
                .get::<shared::components::UnitLevel>(unit)
                .ok_or_else(|| format!("SOURCE_NATIVE_LEVEL_ABSENT:{guid}"))?
                .0;
            Ok((
                guid,
                SourceSnapshot {
                    entry,
                    native_level,
                    scaling: world
                        .get::<shared::level_scaling::LevelScaling>(unit)
                        .copied(),
                },
            ))
        })
        .collect()
}

pub(super) fn apply_source_snapshots(
    world: &mut World,
    sources: &BTreeMap<u32, SourceSnapshot>,
    actors: &BTreeMap<u32, Entity>,
) -> Result<(), String> {
    let current = snapshot_sources(world, actors)?;
    for (&guid, saved) in sources {
        let live = current
            .get(&guid)
            .ok_or_else(|| format!("RECORDED_SOURCE_ABSENT:{guid}"))?;
        if (live.entry, live.native_level) != (saved.entry, saved.native_level) {
            return Err(format!("NATIVE_SOURCE_IDENTITY_OR_LEVEL_CHANGED:{guid}"));
        }
        match (saved.scaling, live.scaling) {
            (Some(recorded), Some(current))
                if (
                    recorded.content_tuning_id,
                    recorded.min_level,
                    recorded.max_level,
                ) == (
                    current.content_tuning_id,
                    current.min_level,
                    current.max_level,
                ) =>
            {
                world.entity_mut(actors[&guid]).insert(recorded);
            }
            (None, None) => {}
            _ => return Err(format!("NATIVE_SOURCE_SCALING_CATALOG_CHANGED:{guid}")),
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(super) struct Frame {
    pub tick: u32,
    pub positions: Vec<Option<[f32; 3]>>,
    pub in_combat: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(super) struct Intake {
    pub schema: u32,
    pub seed: u64,
    pub map_id: u32,
    pub difficulty_id: u32,
    pub level: u8,
    pub end_tick: u32,
    pub source_status: String,
    pub hits: Vec<Hit>,
    pub sources: BTreeMap<u32, SourceSnapshot>,
    pub frames: Vec<Frame>,
    pub pulls: serde_json::Value,
    pub source_deaths: Vec<(u32, u32)>,
    pub gaps: Vec<String>,
}

#[derive(Resource, Default)]
pub(crate) struct Capture {
    pub members: Vec<Entity>,
    pub sources: BTreeMap<Entity, (u32, u32)>,
    pub start_tick: u32,
    pub hits: Vec<Hit>,
    pub consumed_absorbs: BTreeMap<Entity, f64>,
    pub(super) source_snapshots: BTreeMap<u32, SourceSnapshot>,
    pub gaps: Vec<String>,
}

pub(crate) fn record(
    world: &mut World,
    source: Entity,
    target: Entity,
    school: u32,
    raw: f32,
    kind: HitKind,
) {
    if !world.contains_resource::<Capture>() {
        return;
    }
    let now = tick(world);
    let mut capture = world.resource_mut::<Capture>();
    let Some(target_slot) = capture.members.iter().position(|&unit| unit == target) else {
        return;
    };
    let Some(&(source_guid, pack)) = capture.sources.get(&source) else {
        if !capture.members.contains(&source) {
            let gap = "RAW_SOURCE_IDENTITY_UNAVAILABLE".to_string();
            if !capture.gaps.contains(&gap) {
                capture.gaps.push(gap);
            }
        }
        return;
    };
    let tick = now.saturating_sub(capture.start_tick);
    capture.hits.push(Hit {
        tick,
        source_guid,
        pack,
        target_slot,
        school,
        raw,
        kind,
    });
}

pub(crate) fn spell_bonus_source(
    world: &mut World,
    source: Entity,
    bonus_caster: Entity,
    target: Entity,
) -> Option<u32> {
    if source == bonus_caster {
        return None;
    }
    let mut capture = world.get_resource_mut::<Capture>()?;
    if !capture.members.contains(&target) || !capture.sources.contains_key(&source) {
        return None;
    }
    if let Some(&(guid, _)) = capture.sources.get(&bonus_caster) {
        return Some(guid);
    }
    let gap = "DAMAGE_SHIELD_BONUS_CASTER_IDENTITY_UNAVAILABLE".to_string();
    if !capture.gaps.contains(&gap) {
        capture.gaps.push(gap);
    }
    None
}

/// Called only for points the native absorb consumer actually removed.
pub(crate) fn record_absorb(world: &mut World, caster: Entity, target: Entity, amount: f32) {
    let Some(mut capture) = world.get_resource_mut::<Capture>() else {
        return;
    };
    if capture.members.contains(&target) {
        *capture.consumed_absorbs.entry(caster).or_default() += f64::from(amount);
    }
}

pub(crate) fn note_unrecorded_direct(world: &mut World, source: Entity, target: Entity) {
    let Some(mut capture) = world.get_resource_mut::<Capture>() else {
        return;
    };
    if capture.members.contains(&target) && capture.sources.contains_key(&source) {
        let gap = "DIRECT_DAMAGE_WITHOUT_RAW_MITIGATION_BOUNDARY".to_string();
        if !capture.gaps.contains(&gap) {
            capture.gaps.push(gap);
        }
    }
}

pub(super) fn tick(world: &World) -> u32 {
    (world.resource::<Time<Virtual>>().elapsed_secs_f64() / 0.05).round() as u32
}
