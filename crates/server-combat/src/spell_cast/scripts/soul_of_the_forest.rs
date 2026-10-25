//! SimC76f8118 sc_druid.cpp6393–6398,6469–6475,6553: persistent next-heal bonus.
use bevy::prelude::*;

use crate::auras::{ActiveAuras, RemoveReason, handlers, has_aura, remove_auras};
use crate::spell_info::{SpellEffect, SpellInfo};

const TALENT: u32 = 158478;
const BUFF: u32 = 114108;
pub(super) const REGROWTH: u32 = 8936;
pub(super) const REJUVENATION: u32 = 774;
const GERMINATION_HOT: u32 = 155777;

pub(super) fn grant_after_swiftmend(world: &mut World, caster: Entity, depth: u8) {
    if has_aura(world, caster, TALENT) {
        super::super::cast_triggered(world, caster, BUFF, Some(caster), depth + 1);
    }
}

fn persistent_multiplier(world: &World, caster: Entity) -> f32 {
    world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == BUFF && aura.caster == caster)
        })
        .and_then(|aura| aura.effects.iter().find(|effect| effect.index == 0))
        .map_or(1.0, |effect| 1.0 + effect.amount / 100.0)
}

pub(in crate::spell_cast) fn direct_multiplier(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    periodic: bool,
) -> f32 {
    if spell.id == REGROWTH && !periodic {
        persistent_multiplier(world, caster)
    } else {
        1.0
    }
}

pub(crate) fn snapshot_amount(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    effect: &SpellEffect,
    amount: f32,
) -> f32 {
    let eligible = matches!(spell.id, REGROWTH | REJUVENATION | GERMINATION_HOT);
    if eligible && effect.aura == handlers::PERIODIC_HEAL {
        amount * persistent_multiplier(world, caster)
    } else {
        amount
    }
}

pub(super) fn consume_after_heal(world: &mut World, caster: Entity, _target: Entity, _depth: u8) {
    remove_auras(world, caster, RemoveReason::Consumed, |aura| {
        aura.spell_id == BUFF && aura.caster == caster
    });
}
