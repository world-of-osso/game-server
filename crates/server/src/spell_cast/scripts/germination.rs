//! SimC76f8118 sc_druid.cpp6487–6518: redirect Rejuvenation before application.
use bevy::prelude::*;

use crate::auras::{ActiveAuras, has_aura};

const REJUVENATION: u32 = 774;
const GERMINATION_TALENT: u32 = 155675;
const GERMINATION_HOT: u32 = 155777;

pub(in crate::spell_cast) fn redirect_rejuvenation(
    world: &mut World,
    caster: Entity,
    target: Entity,
    spell: u32,
    depth: u8,
) -> bool {
    if spell != REJUVENATION || !has_aura(world, caster, GERMINATION_TALENT) {
        return false;
    }
    let base = own_expiry(world, caster, target, REJUVENATION);
    let germination = own_expiry(world, caster, target, GERMINATION_HOT);
    let use_germination = match (base, germination) {
        (Some(_), None) => true,
        (Some(base), Some(germination)) => germination < base,
        _ => false,
    };
    if !use_germination {
        return false;
    }
    super::super::cast_triggered(world, caster, GERMINATION_HOT, Some(target), depth + 1);
    true
}

fn own_expiry(world: &World, caster: Entity, target: Entity, spell: u32) -> Option<f64> {
    world
        .get::<ActiveAuras>(target)?
        .iter()
        .find(|aura| aura.caster == caster && aura.spell_id == spell)?
        .expires_at
}
