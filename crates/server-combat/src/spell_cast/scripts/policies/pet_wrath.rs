//! Native pet payloads at Bestial Wrath's successful cast boundary.
use bevy::prelude::*;

use crate::spell_cast::{self, effects::EffectCtx, queued_triggers};
use crate::spell_info::SpellInfo;

const PET_BESTIAL_WRATH: u32 = 186254;
const APEX_BESTIAL_WRATH: u32 = 1285912;
const BESTIAL_STRIKE: u32 = 344572;
const APEX_AURA_DELAY_MS: u32 = 450;
const APEX_STRIKE_DELAY_MS: u32 = 1500;

pub(super) fn after_cast(world: &mut World, owner: Entity, spell: &SpellInfo) {
    // SimC76f8118 sc_hunter.cpp6710–6728: permanent pets gain their buff
    // after their strike; Apex gains its distinct buff/strike after450/1500ms.
    let main = crate::hunter_pet::summoned_pet(world, owner);
    for pet in crate::hunter_pet::active_owned_pets(world, owner) {
        if crate::hunter_pet::is_stabled_guardian(world, pet) {
            schedule_apex_wrath(world, pet, spell);
        } else {
            cast_permanent_wrath(world, pet, main);
        }
    }
}

fn cast_permanent_wrath(world: &mut World, pet: Entity, main: Option<Entity>) {
    // The existing native after-cast hook already fired the main strike.
    if Some(pet) != main {
        let target = world
            .get::<crate::networking::CombatTarget>(pet)
            .and_then(|target| target.0);
        if let Some(target) = target {
            spell_cast::cast_triggered(world, pet, BESTIAL_STRIKE, Some(target), 1);
        }
    }
    spell_cast::cast_triggered(world, pet, PET_BESTIAL_WRATH, Some(pet), 1);
}

fn schedule_apex_wrath(world: &mut World, pet: Entity, spell: &SpellInfo) {
    let ctx = EffectCtx {
        caster: pet,
        spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    queued_triggers::schedule_full_trigger(
        world,
        &ctx,
        pet,
        APEX_BESTIAL_WRATH,
        APEX_AURA_DELAY_MS,
    );
    if let Some(target) = spell.cast_primary_target {
        queued_triggers::schedule_full_trigger(
            world,
            &ctx,
            target,
            BESTIAL_STRIKE,
            APEX_STRIKE_DELAY_MS,
        );
    }
}
