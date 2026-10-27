//! Putrefy's real summoned ghoul travels before delivering its native payload.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{script_registry::SpellScriptHook, summon};

const PUTREFY_GHOUL: u32 = 1277098;
const PUTREFY_SINGLE: u32 = 1277016;
const PUTREFY_AREA: u32 = 390220;

pub(super) const HOOKS: &[SpellScriptHook] =
    &[SpellScriptHook::SummonInitialized(start_putrefy_ghoul)];

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
    let now = combat::spell_cast::now(world);
    world.get_mut::<summon::SummonedCreature>(actor).unwrap().expires_at = Some(now);
}
