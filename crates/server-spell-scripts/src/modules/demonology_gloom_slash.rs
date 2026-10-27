//! Mark of Shatug: each actual Gloomhound melee and Headbutt execution casts Gloom Slash.
use super::combat;
use bevy::prelude::*;
use combat::combat::AutoAttack;
use combat::spell_cast;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::summon::SummonedCreature;
use shared::components::Npc;

const MARK_OF_SHATUG: u32 = 455449;
const GLOOMHOUND: u32 = 226268;
const HEADBUTT: u32 = 267999;
const GLOOM_SLASH: u32 = 455491;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterWhiteImpact(after_melee),
    SpellScriptHook::OnCast(after_headbutt),
];

fn marked_gloomhound(world: &World, pet: Entity) -> bool {
    world
        .get::<Npc>(pet)
        .is_some_and(|npc| npc.template_id == GLOOMHOUND)
        && world
            .get::<SummonedCreature>(pet)
            .is_some_and(|summon| combat::auras::has_aura(world, summon.owner, MARK_OF_SHATUG))
}

// SimC76f8118 pets.cpp1361–1373/1405–1418 execute Gloom Slash on the
// attack's target after every melee and Headbutt execution, hit or not.
fn slash(world: &mut World, pet: Entity, depth: u8) {
    if !marked_gloomhound(world, pet) {
        return;
    }
    let Some(target) = world.get::<AutoAttack>(pet).map(|attack| attack.victim) else {
        return;
    };
    spell_cast::cast_triggered(world, pet, GLOOM_SLASH, Some(target), depth + 1);
}

fn after_melee(world: &mut World, pet: Entity) {
    slash(world, pet, 0);
}

fn after_headbutt(world: &mut World, pet: Entity, spell_id: u32, depth: u8) {
    if spell_id == HEADBUTT {
        slash(world, pet, depth);
    }
}
