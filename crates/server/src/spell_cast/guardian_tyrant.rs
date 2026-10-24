//! SimC76f8118 actions3580–3640/pets924–928,954–959,1217–1244.
//! Native1276788 aura79 supplies the per-demon amount and cumulative limit.
use crate::spell_cast::{self as cast, summon::SummonedCreature};
use bevy::prelude::*;
use shared::components::Npc;

const TYRANT: u32 = 265187;
const DEMONIC_POWER: u32 = 1276788;
const FELGUARD: u32 = 17252;

fn is_roster_summon(spell: u32, entry: u32) -> bool {
    super::is_wild_imp(spell, entry) || matches!((spell, entry), (193331 | 193332, 98035))
}

fn is_tyrant(spell: u32, entry: u32) -> bool {
    spell == TYRANT && matches!(entry, 135002 | 250289)
}

pub(crate) fn on_spawn(world: &mut World, actor: Entity) {
    let summon = world.get::<SummonedCreature>(actor).unwrap();
    let (spell, owner) = (summon.spell_id, summon.owner);
    let entry = world.get::<Npc>(actor).unwrap().template_id;
    if is_tyrant(spell, entry) {
        let count = count_owned_demons(world, owner);
        for _ in 0..count {
            apply_stack(world, actor);
        }
    } else if is_roster_summon(spell, entry) {
        for tyrant in read_active_tyrants(world, owner) {
            apply_stack(world, tyrant);
        }
    }
}

fn count_owned_demons(world: &mut World, owner: Entity) -> usize {
    let guardians = world
        .query::<(Entity, &SummonedCreature, &Npc)>()
        .iter(world)
        .filter(|(actor, summon, npc)| {
            summon.owner == owner
                && is_roster_summon(summon.spell_id, npc.template_id)
                && cast::targets::is_alive(world, *actor)
        })
        .count();
    // Pinned SimC's default bug model counts actual active Felguard at Tyrant summon.
    // It does not turn every demon or a later replacement pet into a roster entry.
    let felguards = world
        .query::<(Entity, &crate::hunter_pet::HunterPet, &Npc)>()
        .iter(world)
        .filter(|(actor, pet, npc)| {
            pet.owner == owner
                && npc.template_id == FELGUARD
                && cast::targets::is_alive(world, *actor)
        })
        .count();
    guardians + felguards
}

fn read_active_tyrants(world: &mut World, owner: Entity) -> Vec<Entity> {
    world
        .query::<(Entity, &SummonedCreature, &Npc)>()
        .iter(world)
        .filter(|(actor, summon, npc)| {
            summon.owner == owner
                && is_tyrant(summon.spell_id, npc.template_id)
                && cast::targets::is_alive(world, *actor)
        })
        .map(|(actor, _, _)| actor)
        .collect()
}

fn apply_stack(world: &mut World, tyrant: Entity) {
    cast::cast_triggered(world, tyrant, DEMONIC_POWER, Some(tyrant), 0);
}

pub(crate) fn on_despawn(world: &mut World, actor: Entity) {
    let summon = world.get::<SummonedCreature>(actor).unwrap();
    let owner = summon.owner;
    let entry = world.get::<Npc>(actor).unwrap().template_id;
    if !is_roster_summon(summon.spell_id, entry) {
        return;
    }
    for tyrant in read_active_tyrants(world, owner) {
        let instance = world
            .get::<crate::auras::ActiveAuras>(tyrant)
            .and_then(|auras| auras.iter().find(|aura| aura.spell_id == DEMONIC_POWER))
            .map(|aura| aura.instance_id);
        if let Some(instance) = instance {
            crate::auras::spend_stack(world, tyrant, instance);
        }
    }
}
