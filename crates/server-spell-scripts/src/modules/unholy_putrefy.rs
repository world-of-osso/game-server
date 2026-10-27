//! Putrefy's real summoned ghoul travels before delivering its native payload.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{script_registry::SpellScriptHook, summon};

const PUTREFY_GHOUL: u32 = 1277098;
const PUTREFY_SINGLE: u32 = 1277016;
const PUTREFY_AREA: u32 = 390220;
const LORD_OF_THE_DEAD: u32 = 1256813;
const MAGUS_SUMMON: u32 = 1242294;

#[derive(Component)]
struct PutrefyImpacted;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::SummonInitialized(start_putrefy_ghoul),
    SpellScriptHook::SummonExpired(raise_magus),
];

fn start_putrefy_ghoul(world: &mut World, actor: Entity) {
    let Some(summon) = world.get::<summon::SummonedCreature>(actor) else {
        return;
    };
    if summon.spell_id != PUTREFY_GHOUL {
        return;
    }
    let owner = summon.owner;
    let Some(target) = world
        .get::<combat::networking::CombatTarget>(owner)
        .and_then(|target| target.0)
    else {
        warn!(?owner, "Putrefy ghoul has no selected target");
        return;
    };
    // SimC76f8118 sc_death_knight.cpp3711–3740: the actual ghoul travels,
    // then its owner delivers Putrefy's native payload and dismisses it.
    summon::start_melee_attack(world, actor, target);
    world.entity_mut(actor).insert(summon::SummonArrival {
        target,
        on_arrival: impact_and_dismiss,
    });
}

fn impact_and_dismiss(world: &mut World, actor: Entity, target: Entity) {
    let owner = world.get::<summon::SummonedCreature>(actor).unwrap().owner;
    for spell in [PUTREFY_SINGLE, PUTREFY_AREA] {
        if !combat::spell_cast::try_cast_triggered(world, owner, spell, Some(target), 1) {
            warn!(?owner, ?target, spell, "Native Putrefy impact refused");
        }
    }
    world.entity_mut(actor).insert(PutrefyImpacted);
    let now = combat::spell_cast::now(world);
    world
        .get_mut::<summon::SummonedCreature>(actor)
        .unwrap()
        .expires_at = Some(now);
}

fn raise_magus(world: &mut World, actor: Entity) {
    if world.get::<PutrefyImpacted>(actor).is_none() {
        return;
    }
    let owner = world.get::<summon::SummonedCreature>(actor).unwrap().owner;
    if !combat::auras::has_aura(world, owner, LORD_OF_THE_DEAD) {
        return;
    }
    // SimC76f8118 sc_death_knight.cpp3716–3743: the native Lord chance
    // summons a Magus only after the putrefied ghoul is dismissed.
    let chance = combat::spell_cast::scripts::script_effect_value(world, owner, LORD_OF_THE_DEAD, 0);
    if combat::random::random::<f32>() * 100.0 >= chance {
        return;
    }
    world.commands().queue(move |world: &mut World| {
        if !combat::spell_cast::try_cast_triggered(world, owner, MAGUS_SUMMON, Some(owner), 1) {
            warn!(?owner, "Native Lord of the Dead Magus summon refused");
        }
    });
}
