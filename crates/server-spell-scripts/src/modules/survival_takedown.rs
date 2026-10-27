//! Actual Takedown's owner and main-pet children retain their native payloads.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] =
    &[SpellScriptHook::SuccessfulCastRelationships(after_cast)];

const TAKEDOWN: u32 = 1250646;
const PLAYER_DAMAGE: u32 = 1253859;
const PET_DAMAGE: u32 = 1253862;

fn after_cast(world: &mut World, caster: Entity, spell: &SpellInfo) {
    // SimC76f8118 sc_hunter.cpp6396–6405: the native Takedown buff and
    // shared forced mark already exist before these two distinct actor impacts.
    if spell.id != TAKEDOWN {
        return;
    }
    let Some(target) = spell.cast_primary_target else {
        return;
    };
    spell_cast::cast_triggered(world, caster, PLAYER_DAMAGE, Some(target), 1);
    cast_pet_damage(world, caster, target);
}

fn cast_pet_damage(world: &mut World, owner: Entity, target: Entity) {
    let Some(pet) = combat::hunter_pet::summoned_pet(world, owner) else {
        return;
    };
    if !spell_cast::targets::is_alive(world, pet) {
        return;
    }
    let Some(mut spell) = spell_cast::load_triggered_spell(world, PET_DAMAGE, 1) else {
        return;
    };
    // sc_hunter.cpp3096–3105: this pet payload's native energize effect is
    // not reflected in-game; only its authored damage effect executes here.
    spell.effects.retain(|effect| effect.index != 1);
    spell_cast::launch_triggered_spell(world, pet, &spell, Some(target), 1);
}
