//! SimC76f8118 sc_shaman.cpp4564–4573/6134–6135/12758–12822:
//! each live Crash stack retains its Short Circuit snapshot.
use std::collections::VecDeque;

use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, AuraInstance, RemoveReason};
use combat::class_progression::KnownSpells;

const FOUR_PIECE: u32 = 1296628;
const CRASH_STACKS: u32 = 1252415;
const APEX: u32 = 1252373;

#[derive(Component, Default)]
struct Snapshots(VecDeque<f32>);

pub(super) fn retain(world: &mut World, caster: Entity, multiplier: f32) {
    if !auras::has_aura(world, caster, FOUR_PIECE) {
        return;
    }
    let asynchronous = world
        .get::<KnownSpells>(caster)
        .is_some_and(|known| known.contains(APEX));
    let at_cap = world.get::<ActiveAuras>(caster).is_some_and(|active| {
        active.iter().any(|aura| {
            aura.spell_id == CRASH_STACKS
                && aura.caster == caster
                && aura.max_stacks > 0
                && aura.stacks >= aura.max_stacks
        })
    });
    if world.get::<Snapshots>(caster).is_none() {
        world.entity_mut(caster).insert(Snapshots::default());
    }
    let mut snapshots = world.get_mut::<Snapshots>(caster).unwrap();
    if !asynchronous {
        snapshots.0.clear();
    } else if at_cap {
        snapshots.0.pop_front();
    }
    snapshots.0.push_back(multiplier - 1.0);
}

pub(super) fn multiplier(world: &World, caster: Entity) -> f32 {
    let Some(snapshots) = world.get::<Snapshots>(caster) else {
        return 1.0;
    };
    if snapshots.0.is_empty() {
        return 1.0;
    }
    1.0 + snapshots.0.iter().sum::<f32>() / snapshots.0.len() as f32
}

pub(super) fn expire_stacks(world: &mut World, caster: Entity, _aura: &AuraInstance, lost: u8) {
    if let Some(mut snapshots) = world.get_mut::<Snapshots>(caster) {
        for _ in 0..lost {
            snapshots.0.pop_front();
        }
    }
}

pub(super) fn clear(
    world: &mut World,
    caster: Entity,
    _aura: &AuraInstance,
    _reason: RemoveReason,
) {
    if let Some(mut snapshots) = world.get_mut::<Snapshots>(caster) {
        snapshots.0.clear();
    }
}
