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
    pub frames: Vec<Frame>,
    pub pulls: serde_json::Value,
    pub gaps: Vec<String>,
}

#[derive(Resource, Default)]
pub(crate) struct Capture {
    pub members: Vec<Entity>,
    pub sources: BTreeMap<Entity, (u32, u32)>,
    pub start_tick: u32,
    pub hits: Vec<Hit>,
    pub gaps: Vec<String>,
}

pub(super) fn tick(world: &World) -> u32 {
    (world.resource::<Time<Virtual>>().elapsed_secs_f64() / 0.05).round() as u32
}
