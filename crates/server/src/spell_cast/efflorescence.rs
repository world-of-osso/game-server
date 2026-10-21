//! TC a352b1fa spell_druid.cpp719–778; Spell.cpp SelectRandomInjuredTargets.
use bevy::prelude::*;
use rand::seq::SliceRandom;
use shared::components::{Health, Player, UnitSummonedBy};
use std::collections::BTreeSet;

use super::summon::SummonedCreature;

const EFFLORESCENCE: u32 = 145205;
pub(crate) const DUMMY: u32 = 81262;
pub(super) const HEAL: u32 = 81269;
const HEAL_TARGETS: usize = 3;

/// Capture only this cast's new summons at the synchronous effect/AfterCast boundary.
/// This runtime does not retain TC's per-cast ExecuteLog summon target list.
pub(super) fn read_previous_summons(
    world: &mut World,
    caster: Entity,
    spell: u32,
) -> Option<BTreeSet<Entity>> {
    (spell == EFFLORESCENCE).then(|| read_owned_summons(world, caster))
}

fn read_owned_summons(world: &mut World, caster: Entity) -> BTreeSet<Entity> {
    world
        .query::<(Entity, &SummonedCreature)>()
        .iter(world)
        .filter(|(_, summon)| summon.owner == caster && summon.spell_id == EFFLORESCENCE)
        .map(|(unit, _)| unit)
        .collect()
}

pub(super) fn initialize_new_summons(
    world: &mut World,
    caster: Entity,
    previous: Option<BTreeSet<Entity>>,
) {
    let Some(previous) = previous else { return };
    let current = read_owned_summons(world, caster);
    for &unit in current.difference(&previous) {
        if let Err(failure) = super::creature_cast(world, unit, DUMMY, Some(unit), false) {
            debug!(?unit, ?failure, "Efflorescence summon aura refused");
        }
    }
}

/// Source uses only DONT_REPORT_CAST_ERROR, not FULL_MASK: retain native checks.
pub(crate) fn pulse(
    world: &mut World,
    unit: Entity,
    _aura: &crate::auras::AuraInstance,
    index: u8,
) {
    if index != 0 {
        return;
    }
    let Some(owner) = world
        .get::<UnitSummonedBy>(unit)
        .and_then(|owner| Entity::try_from_bits(owner.0))
    else {
        return;
    };
    if let Err(failure) = super::creature_cast(world, owner, HEAL, Some(unit), false) {
        debug!(?owner, ?failure, "Efflorescence native pulse refused");
    }
}

pub(super) fn filter_heal_targets(world: &World, caster: Entity, units: &mut Vec<Entity>) {
    if units.len() <= HEAL_TARGETS {
        return;
    }
    units.sort_by_key(|unit| read_target_priority(world, caster, *unit));
    let boundary = read_target_priority(world, caster, units[HEAL_TARGETS - 1]);
    let first = units.partition_point(|unit| read_target_priority(world, caster, *unit) < boundary);
    let end = units.partition_point(|unit| read_target_priority(world, caster, *unit) <= boundary);
    units[first..end].shuffle(&mut rand::thread_rng());
    units.truncate(HEAL_TARGETS);
}

fn read_target_priority(world: &World, caster: Entity, unit: Entity) -> (bool, bool, bool) {
    let full = world
        .get::<Health>(unit)
        .is_none_or(|health| health.current >= health.max);
    // NPC static-flag4 treat-as-raid classification has no runtime producer yet.
    let non_player = world.get::<Player>(unit).is_none();
    let ungrouped = !super::targets::shares_raid(world, caster, unit);
    (full, non_player, ungrouped)
}
