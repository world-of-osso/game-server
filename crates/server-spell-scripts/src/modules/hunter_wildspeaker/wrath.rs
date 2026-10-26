//! Wildspeaker's native buff follows real owner BW and actual new DireCritters.
use super::{DIRE_CRITTER_SUMMONS, WILDSPEAKER, combat, has_talent, read_owned_critters};
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::spell_cast::{self, TriggeredSpellOverrides};
use combat::spell_info::SpellInfo;

const BESTIAL_WRATH: u32 = 19574;
const CRITTER_WRATH: u32 = 1235388;

#[derive(Component)]
struct WrathInitialized;

fn read_owner_remaining(world: &World, owner: Entity) -> Option<i32> {
    let now = spell_cast::now(world);
    world
        .get::<ActiveAuras>(owner)?
        .iter()
        .find(|aura| aura.spell_id == BESTIAL_WRATH && aura.caster == owner)?
        .expires_at
        .filter(|deadline| *deadline > now)
        .map(|deadline| ((deadline - now) * 1000.0).round() as i32)
}

fn apply_wrath(world: &mut World, pet: Entity, duration_ms: Option<i32>) {
    let values = TriggeredSpellOverrides {
        duration_ms,
        ..Default::default()
    };
    let depth = spell_cast::next_trigger_depth(world);
    if !spell_cast::cast_triggered_with_overrides(
        world,
        pet,
        CRITTER_WRATH,
        Some(pet),
        depth,
        values,
    ) {
        warn!(?pet, "Native Wildspeaker Wrath refused");
    }
}

fn inherit_new(world: &mut World, owner: Entity, summon_spell: u32) {
    let duration = read_owner_remaining(world, owner);
    let pets: Vec<_> = read_owned_critters(world, owner)
        .into_iter()
        .filter(|(pet, spell)| {
            *spell == summon_spell && world.get::<WrathInitialized>(*pet).is_none()
        })
        .map(|(pet, _)| pet)
        .collect();
    for pet in pets {
        world.entity_mut(pet).insert(WrathInitialized);
        // SimC76f8118 hunter1866–1867: only a newly joining DireCritter
        // inherits the remaining owner BW; older actors are not refreshed.
        if let Some(milliseconds) = duration {
            apply_wrath(world, pet, Some(milliseconds));
        }
    }
}

pub(super) fn after_launch(world: &mut World, owner: Entity, spell: &SpellInfo, foreground: bool) {
    if !has_talent(world, owner, WILDSPEAKER) {
        return;
    }
    if foreground && spell.id == BESTIAL_WRATH {
        // SimC76f8118 hunter6729–6736: existing DireCritters get a fresh
        // native1235388 buff. Its own authored effects/duration supply values.
        for (pet, _) in read_owned_critters(world, owner) {
            apply_wrath(world, pet, None);
        }
    } else if DIRE_CRITTER_SUMMONS.contains(&spell.id) {
        inherit_new(world, owner, spell.id);
    }
}
