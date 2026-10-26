//! Stable-body white impacts grow already-earned native Wyvern Cry.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras};
use combat::spell_cast::script_registry::SpellScriptHook;

#[path = "hunter_wyvern/fury.rs"]
mod fury;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterWhiteImpact(grow_cry),
    SpellScriptHook::EarlyAuraRegistration(fury::register),
    SpellScriptHook::AfterElementalLaunch(fury::extend_after_command),
];
const WYVERN_CRY: u32 = 471881;

fn grow_cry(world: &mut World, pet: Entity) {
    // SimC76f8118 sc_hunter.cpp2974–3010: main, companion and the
    // stable Nature's Ally share main_pet_base_melee_t. Other guardian
    // bodies do not produce these stacks, and no inactive Cry is started.
    let Some(owner) = world
        .get::<combat::hunter_pet::HunterPet>(pet)
        .map(|pet| pet.owner)
    else {
        return;
    };
    let stacks = world.get::<ActiveAuras>(owner).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == WYVERN_CRY && aura.caster == owner)
            .map(|aura| aura.stacks)
    });
    if let Some(stacks) = stacks {
        // SetStackAmount keeps the native cap, expiration and tick phase.
        auras::set_owned_stacks(world, owner, owner, WYVERN_CRY, stacks.saturating_add(1));
    }
}
