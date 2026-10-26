//! SimC76f8118 sc_druid.cpp6393–6398,6469–6475,6553: persistent next-heal bonus.
use bevy::prelude::*;

use super::combat;
use combat::auras::{ActiveAuras, RemoveReason, handlers, has_aura, remove_auras};
use combat::spell_cast::script_registry::{HitHook, SpellScriptHook};
use combat::spell_info::{SpellEffect, SpellInfo};

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::HealingMultiplier(direct_multiplier),
    SpellScriptHook::SnapshotHealingAura(snapshot_amount),
    SpellScriptHook::Hit(hit_hook),
];

fn hit_hook(spell: u32) -> Option<HitHook> {
    (spell == REJUVENATION).then_some(consume_after_heal)
}

const TALENT: u32 = 158478;
const BUFF: u32 = 114108;
pub(super) const REGROWTH: u32 = 8936;
pub(super) const REJUVENATION: u32 = 774;
const GERMINATION_HOT: u32 = 155777;

pub(super) fn grant_after_swiftmend(world: &mut World, caster: Entity, depth: u8) {
    if has_aura(world, caster, TALENT) {
        combat::spell_cast::cast_triggered(world, caster, BUFF, Some(caster), depth + 1);
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

fn direct_multiplier(world: &World, caster: Entity, spell: &SpellInfo, periodic: bool) -> f32 {
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
