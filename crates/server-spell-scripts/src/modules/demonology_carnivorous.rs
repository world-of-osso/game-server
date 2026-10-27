//! Carnivorous Stalkers queue Dreadbite from actual Dreadstalker melee swings.
use super::combat;
use bevy::prelude::*;
use combat::combat::AutoAttack;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::summon::SummonedCreature;
use combat::spell_cast::{self, SpellHistory};
use combat::spell_info::SpellStore;
use shared::casting::CastState;
use shared::components::Npc;

const CARNIVOROUS_STALKERS: u32 = 386194;
const DREADBITE: u32 = 271971;
const DREADSTALKER: u32 = 98035;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterWhiteImpact(roll_extra_bite),
    SpellScriptHook::AfterQueuedTriggers(cast_earned_bites),
];

#[derive(Component, Default)]
struct EarnedBites(u32);

fn roll_extra_bite(world: &mut World, pet: Entity) {
    if world
        .get::<Npc>(pet)
        .is_none_or(|npc| npc.template_id != DREADSTALKER)
    {
        return;
    }
    let Some(owner) = world
        .get::<SummonedCreature>(pet)
        .map(|summon| summon.owner)
    else {
        return;
    };
    if !combat::auras::has_aura(world, owner, CARNIVOROUS_STALKERS) {
        return;
    }
    // SimC76f8118 pets.cpp1116–1133 and init.cpp1272–1274 roll once
    // per actual melee execution, including non-hits, at native effect1's rate.
    let chance = spell_cast::scripts::script_effect_value(world, owner, CARNIVOROUS_STALKERS, 0);
    if combat::random::random::<f32>() * 100.0 < chance {
        world
            .entity_mut(pet)
            .entry::<EarnedBites>()
            .or_default()
            .get_mut()
            .0 += 1;
    }
}

fn cast_earned_bites(world: &mut World) {
    let pets: Vec<_> = world
        .query::<(Entity, &EarnedBites)>()
        .iter(world)
        .filter(|(_, earned)| earned.0 > 0)
        .map(|(pet, _)| pet)
        .collect();
    for pet in pets {
        if world.get::<CastState>(pet).is_some() {
            continue;
        }
        let Some(target) = world.get::<AutoAttack>(pet).map(|attack| attack.victim) else {
            continue;
        };
        let spell = world
            .resource::<SpellStore>()
            .get(DREADBITE)
            .expect("native Dreadbite")
            .clone();
        let now = spell_cast::now(world);
        if world
            .get::<SpellHistory>(pet)
            .is_some_and(|history| history.on_gcd(&spell, now) || history.on_cooldown(&spell, now))
        {
            continue;
        }
        match spell_cast::creature_cast(world, pet, DREADBITE, Some(target), false) {
            Ok(()) => {
                world
                    .get_mut::<EarnedBites>(pet)
                    .expect("earned bite queue")
                    .0 -= 1
            }
            Err(error) => debug!(?pet, ?target, ?error, "Earned Dreadbite not ready"),
        }
    }
}
