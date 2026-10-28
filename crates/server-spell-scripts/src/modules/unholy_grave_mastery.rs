//! Selected Grave Mastery applies its native critical-damage aura to real pets.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{self, script_registry::SpellScriptHook, summon::SummonedCreature};
use shared::components::Npc;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::SummonInitialized(initialize_summon),
    SpellScriptHook::AfterCastUpdate(initialize_controlled_ghoul),
];

fn initialize_summon(world: &mut World, pet: Entity) {
    let Some(summon) = world.get::<SummonedCreature>(pet) else { return; };
    let Some(npc) = world.get::<Npc>(pet) else { return; };
    if !matches!((summon.spell_id, npc.template_id),
        (275430 | 1294026, 237409) | (317776 | 1242294, 163366)
        | (1292072, 264321) | (288853, 149555)
        | (444248, 221632) | (444251, 221633) | (444252, 221634) | (444254, 221635)) { return; }
    apply_mastery(world, summon.owner, pet);
}

fn initialize_controlled_ghoul(world: &mut World) {
    let pets: Vec<_> = world.query::<(Entity, &combat::hunter_pet::HunterPet, &Npc)>().iter(world)
        .filter_map(|(pet, relation, npc)| (npc.template_id == 26125).then_some((pet, relation.owner)))
        .collect();
    for (pet, owner) in pets { apply_mastery(world, owner, pet); }
}

fn apply_mastery(world: &mut World, owner: Entity, pet: Entity) {
    if !combat::auras::has_aura(world, owner, 1238900) || combat::auras::has_aura(world, pet, 1238902) { return; }
    // SimC76f8118 sc_death_knight.cpp2730–2735/2788–2792: Grave Mastery
    // multiplies the actual pet's critical damage using its native aura.
    if !spell_cast::try_cast_triggered(world, pet, 1238902, Some(pet), 1) {
        warn!(?pet, "native Grave Mastery pet aura refused");
    }
}
