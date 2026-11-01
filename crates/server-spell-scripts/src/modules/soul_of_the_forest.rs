//! SimC76f8118 sc_druid.cpp6393–6398,6469–6475,6553: persistent next-heal bonus.
//! Power of the Archdruid 392302 (TC spell_druid.cpp `spell_dru_power_of_the_archdruid`):
//! the consuming Regrowth/Rejuvenation is recast on up to s1 random allies within s2
//! yards of its target. The copies keep the bonus; the buff falls after them.
use bevy::prelude::*;

use super::combat;
use combat::auras::{ActiveAuras, RemoveReason, handlers, has_aura, remove_auras};
use combat::random::{self as rand, seq::SliceRandom};
use combat::spell_cast::script_registry::{HitHook, SpellScriptHook};
use combat::spell_cast::scripts::script_effect_value;
use combat::spell_cast::summon::SummonedCreature;
use combat::spell_cast::targets::{self, Relation};
use combat::spell_info::{SpellEffect, SpellInfo};
use shared::components::Health;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::HealingMultiplier(direct_multiplier),
    SpellScriptHook::SnapshotHealingAura(snapshot_amount),
    SpellScriptHook::Hit(hit_hook),
];

fn hit_hook(spell: u32) -> Option<HitHook> {
    (spell == REJUVENATION).then_some(consume_after_rejuvenation)
}

fn consume_after_rejuvenation(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    consume_after_heal(world, caster, target, REJUVENATION, depth);
}

const TALENT: u32 = 158478;
const BUFF: u32 = 114108;
pub(super) const REGROWTH: u32 = 8936;
pub(super) const REJUVENATION: u32 = 774;
const GERMINATION_HOT: u32 = 155777;
const POWER_OF_THE_ARCHDRUID: u32 = 392302;

/// Marks a caster while Power of the Archdruid recasts, so the copies neither
/// consume the buff nor spend Clearcasting.
#[derive(Component)]
struct ArchdruidCopies;

pub(super) fn is_archdruid_copy(world: &World, caster: Entity) -> bool {
    world.get::<ArchdruidCopies>(caster).is_some()
}

pub(super) fn grant_after_swiftmend(world: &mut World, caster: Entity, depth: u8) {
    if has_aura(world, caster, TALENT) {
        combat::spell_cast::cast_triggered(world, caster, BUFF, Some(caster), depth + 1);
    }
}

fn has_own_buff(world: &World, caster: Entity) -> bool {
    world.get::<ActiveAuras>(caster).is_some_and(|auras| {
        auras
            .iter()
            .any(|aura| aura.spell_id == BUFF && aura.caster == caster)
    })
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

pub(super) fn consume_after_heal(
    world: &mut World,
    caster: Entity,
    target: Entity,
    spell: u32,
    depth: u8,
) {
    if is_archdruid_copy(world, caster) || !has_own_buff(world, caster) {
        return;
    }
    if has_aura(world, caster, POWER_OF_THE_ARCHDRUID) {
        recast_on_nearby_allies(world, caster, target, spell, depth);
    }
    remove_auras(world, caster, RemoveReason::Consumed, |aura| {
        aura.spell_id == BUFF && aura.caster == caster
    });
}

fn recast_on_nearby_allies(
    world: &mut World,
    caster: Entity,
    target: Entity,
    spell: u32,
    depth: u8,
) {
    let Some(center) = targets::position(world, target) else {
        return;
    };
    let limit = script_effect_value(world, caster, POWER_OF_THE_ARCHDRUID, 0) as usize;
    let radius = script_effect_value(world, caster, POWER_OF_THE_ARCHDRUID, 1);
    let mut allies = targets::area_units(world, caster, center, radius, Relation::Raid, 0);
    allies.retain(|&unit| {
        unit != target
            && world.get::<SummonedCreature>(unit).is_none()
            && world
                .get::<Health>(unit)
                .is_some_and(|health| health.current > 0.0)
    });
    allies.shuffle(&mut rand::thread_rng());
    allies.truncate(limit);
    world.entity_mut(caster).insert(ArchdruidCopies);
    for ally in allies {
        combat::spell_cast::cast_triggered(world, caster, spell, Some(ally), depth + 1);
    }
    world.entity_mut(caster).remove::<ArchdruidCopies>();
}
