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

fn expire_instance_stacks(aura: &mut AuraInstance, now: f64) -> usize {
    let before = aura.stack_expirations.len();
    aura.stack_expirations.retain(|&expiry| expiry > now);
    let expired = before - aura.stack_expirations.len();
    if expired == 0 {
        return 0;
    }
    aura.stacks = aura
        .stacks
        .saturating_sub(expired.min(usize::from(u8::MAX)) as u8);
    if aura.stacks == 0 {
        // Whole-aura removal still owns replication and derived-stat cleanup.
        aura.expires_at = Some(now);
    }
    expired
}

pub(super) fn expire_async_stacks(world: &mut World, unit: Entity, now: f64) {
    let Some(mut auras) = world.get_mut::<ActiveAuras>(unit) else {
        return;
    };
    let mut expired = Vec::new();
    let mut changed = false;
    for aura in &mut auras.auras {
        let count = expire_instance_stacks(aura, now);
        if count > 0 {
            changed |= aura.stacks > 0;
            expired.push((aura.clone(), count));
        }
    }
    auras.dirty |= changed;
    drop(auras);
    if changed {
        super::auras_changed(world, unit);
    }
    // SimC expiration_t::execute decrements each independent application. Do not
    // coalesce coincident deadlines into one script callback or duplicate the last.
    for (aura, count) in expired {
        for _ in 0..count {
            super::scripts::run_stack_expire(world, unit, &aura);
        }
    }
}
