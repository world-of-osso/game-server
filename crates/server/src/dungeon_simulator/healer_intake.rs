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
    /// Native damage multiplier / victim health multiplier, before integer rounding.
    pub level_scaling_factor: f32,
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
    _world: &mut World,
    _sources: &BTreeMap<u32, SourceSnapshot>,
    _actors: &BTreeMap<u32, Entity>,
) -> Result<(), String> {
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

#[derive(Clone, Copy, Default)]
pub(super) struct ConsumedAbsorbPoints {
    pub total: f64,
    pub non_unit: f64,
    pub unknown_scale: f64,
}

pub(super) fn non_unit_hits(hits: &[Hit]) -> usize {
    hits.iter()
        .filter(|hit| hit.level_scaling_factor != 1.0)
        .count()
}

#[derive(Resource, Default)]
pub(crate) struct Capture {
    pub members: Vec<Entity>,
    pub sources: BTreeMap<Entity, (u32, u32)>,
    pub start_tick: u32,
    pub hits: Vec<Hit>,
    pub(super) consumed_absorbs: BTreeMap<Entity, ConsumedAbsorbPoints>,
    pub(super) active_hit_scale: Option<(Entity, f32)>,
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
    let level_scaling_factor = crate::level_scaling::damage_to_hp_scale(world, source, target);
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
    if !level_scaling_factor.is_finite() || level_scaling_factor <= 0.0 {
        capture.active_hit_scale = None;
        let gap = "NATIVE_LEVEL_SCALING_FACTOR_INVALID".to_string();
        if !capture.gaps.contains(&gap) {
            capture.gaps.push(gap);
        }
        return;
    }
    capture.active_hit_scale = Some((target, level_scaling_factor));
    let tick = now.saturating_sub(capture.start_tick);
    capture.hits.push(Hit {
        tick,
        source_guid,
        pack,
        target_slot,
        school,
        raw,
        level_scaling_factor,
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
        let scale = capture
            .active_hit_scale
            .filter(|(unit, _)| *unit == target)
            .map(|(_, scale)| scale);
        let usage = capture.consumed_absorbs.entry(caster).or_default();
        let points = f64::from(amount);
        usage.total += points;
        match scale {
            Some(scale) if scale == 1.0 => {}
            Some(_) => usage.non_unit += points,
            None => usage.unknown_scale += points,
        }
    }
}

pub(crate) fn note_unrecorded_direct(world: &mut World, source: Entity, target: Entity) {
    let Some(mut capture) = world.get_resource_mut::<Capture>() else {
        return;
    };
    if capture.members.contains(&target) && capture.sources.contains_key(&source) {
        capture.active_hit_scale = None;
        let gap = "DIRECT_DAMAGE_WITHOUT_RAW_MITIGATION_BOUNDARY".to_string();
        if !capture.gaps.contains(&gap) {
            capture.gaps.push(gap);
        }
    }
}

pub(super) fn tick(world: &World) -> u32 {
    (world.resource::<Time<Virtual>>().elapsed_secs_f64() / 0.05).round() as u32
}
