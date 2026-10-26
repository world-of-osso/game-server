//! SimC76f8118 sc_demon_hunter.cpp4976–4982,9153–9172,9265–9277,12645–12671.
use std::collections::BTreeSet;

use bevy::prelude::*;

use super::combat;
use combat::auras::scripts::AuraScripts;
use combat::auras::{self, AuraInstance, RemoveReason};

const TALENT: u32 = 452402;
const META: u32 = 162264;
const INTENSITY: u32 = 452489;
const STACKS: u32 = 452416;
const ABILITIES: &[u32] = &[201427, 210152, 258920, 452497, 452490];

#[derive(Component, Default)]
struct DemonsurgeUses(BTreeSet<u32>);

fn ability(spell: u32) -> u32 {
    match spell {
        452486 => 210152,
        452487 => 201427,
        456640 => 258920,
        198013 => 452497,
        204596 => 452490,
        spell => spell,
    }
}

pub fn havoc_demonsurge_available(world: &World, caster: Entity, spell: u32) -> bool {
    super::has_talent(world, caster, TALENT)
        && auras::has_aura(world, caster, META)
        && world
            .get::<DemonsurgeUses>(caster)
            .is_some_and(|uses| uses.0.contains(&ability(spell)))
}

pub(super) fn arm_demonic(world: &mut World, caster: Entity) {
    if super::has_talent(world, caster, TALENT) {
        world
            .entity_mut(caster)
            .entry::<DemonsurgeUses>()
            .or_default()
            .into_mut()
            .0
            .extend([201427, 210152]);
    }
}

pub(super) fn before_cast(world: &mut World, caster: Entity, spell: u32, depth: u8) {
    if !super::has_talent(world, caster, TALENT) {
        return;
    }
    if spell == 191427 {
        world
            .entity_mut(caster)
            .entry::<DemonsurgeUses>()
            .or_default()
            .into_mut()
            .0
            .extend(ABILITIES.iter().copied());
        auras::remove_auras(world, caster, RemoveReason::Cancelled, |aura| {
            aura.spell_id == STACKS && aura.caster == caster
        });
        if super::has_talent(world, caster, 452415) {
            combat::spell_cast::cast_triggered(world, caster, INTENSITY, Some(caster), depth + 1);
        }
        return;
    }
    if !havoc_demonsurge_available(world, caster, spell) {
        return;
    }
    world
        .get_mut::<DemonsurgeUses>(caster)
        .expect("available native Demonsurge use")
        .0
        .remove(&ability(spell));
    // Native trigger rows own the source-defined delay and explosion payload:
    //1238696 for Death Sweep,453323 for the other abilities. Their ordinary
    // queued effect64 path resolves native452416's caster-centred area geometry.
    let trigger = if ability(spell) == 210152 {
        1238696
    } else {
        453323
    };
    if !combat::spell_cast::try_cast_triggered(world, caster, trigger, Some(caster), depth + 1) {
        error!(trigger, "Native Demonsurge trigger refused");
    }
}

pub(crate) fn register_havoc(scripts: &mut AuraScripts) {
    scripts.0.entry(META).or_default().on_remove = Some(expire_meta);
}

fn expire_meta(world: &mut World, unit: Entity, aura: &AuraInstance, reason: RemoveReason) {
    if aura.caster != unit {
        return;
    }
    if let Some(mut uses) = world.get_mut::<DemonsurgeUses>(unit) {
        uses.0.clear();
    }
    auras::remove_auras(world, unit, reason, |active| {
        active.caster == unit && matches!(active.spell_id, INTENSITY | STACKS)
    });
}
