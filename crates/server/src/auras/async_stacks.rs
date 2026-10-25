//! SimC76f8118 data_enums.hh1910/buff.cpp777–782: native flag490.
//! Each application expires separately; overflow replaces oldest deadlines
//! (buff.cpp2649–2692). No spell-ID whitelist or invented duration.
use bevy::prelude::*;

use super::{ActiveAuras, AuraInstance};
use crate::spell_info::SpellInfo;

const ATTR15_ASYNCHRONOUS_STACKING_AURA: u32 = 0x0000_0400;

pub(super) fn initial_expirations(
    spell: &SpellInfo,
    max_stacks: u8,
    stacks: u8,
    expires_at: Option<f64>,
) -> Vec<f64> {
    let asynchronous = super::attributes(spell, 15) & ATTR15_ASYNCHRONOUS_STACKING_AURA != 0;
    if !asynchronous || max_stacks <= 1 {
        return Vec::new();
    }
    expires_at.map_or_else(Vec::new, |expiry| vec![expiry; usize::from(stacks)])
}

pub(super) fn merge_expirations(existing: &[f64], incoming: &[f64], overflow: usize) -> Vec<f64> {
    let mut expirations: Vec<_> = existing
        .iter()
        .skip(overflow)
        .chain(incoming)
        .copied()
        .collect();
    expirations.sort_by(f64::total_cmp);
    expirations
}

fn expire_instance_stacks(aura: &mut AuraInstance, now: f64) -> bool {
    let before = aura.stack_expirations.len();
    aura.stack_expirations.retain(|&expiry| expiry > now);
    let expired = before - aura.stack_expirations.len();
    if expired == 0 {
        return false;
    }
    aura.stacks = aura
        .stacks
        .saturating_sub(expired.min(usize::from(u8::MAX)) as u8);
    if aura.stacks == 0 {
        // Ordinary aura removal owns callbacks, replication and derived-stat cleanup.
        aura.expires_at = Some(now);
        return false;
    }
    true
}

pub(super) fn expire_async_stacks(world: &mut World, unit: Entity, now: f64) {
    let Some(mut auras) = world.get_mut::<ActiveAuras>(unit) else {
        return;
    };
    let mut changed = false;
    let mut losses = Vec::new();
    for aura in &mut auras.auras {
        let before = aura.stacks;
        changed |= expire_instance_stacks(aura, now);
        if before > aura.stacks {
            losses.push((aura.clone(), before - aura.stacks));
        }
    }
    auras.dirty |= changed;
    drop(auras);
    for (aura, lost) in losses {
        super::scripts::run_stack_loss(world, unit, &aura, lost);
    }
    if changed {
        super::auras_changed(world, unit);
    }
}
