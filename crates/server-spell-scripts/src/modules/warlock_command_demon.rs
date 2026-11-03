//! Command Demon forwards the owner's native command to the active pet's learned spell.
use bevy::prelude::*;
use shared::protocol::{ACT_ENABLED, PetAction, pet_action_button};

use super::combat;
use combat::spell_cast::script_registry::{HitHook, SpellScriptHook};

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::EffectHit(effect_hit)];

fn effect_hit(spell: u32, effect: u8) -> Option<HitHook> {
    (matches!(spell, 119910 | 119914) && effect == 0).then_some(command_pet)
}

fn command_pet(world: &mut World, owner: Entity, target: Entity, _depth: u8) {
    let command = combat::spell_cast::resolving_spell_id(world, owner)
        .expect("Command Demon requires its resolving spell");
    let Some((pet, spell)) = combat::pet_control::command_demon_pet(world, owner, command) else {
        return;
    };
    let action = PetAction {
        pet: pet.to_bits(),
        action: pet_action_button(spell, ACT_ENABLED),
        target: Some(target.to_bits()),
        position: None,
    };
    if !combat::pet_control::cast_ordered_spell(world, owner, pet, spell, &action) {
        debug!(?owner, ?pet, spell, "Command Demon pet cast refused");
    }
}
