//! Pack Mentality rewards each actual consumed Howl beast with native recovery.
use super::{READY, combat, has_talent};
use bevy::prelude::*;
use combat::spell_cast::scripts::{reduce_cooldown, script_effect_value};

const PACK_MENTALITY: u32 = 472358;
const ALPHA_PREDATOR: u32 = 269737;
const BEAST_COMMAND: u32 = 83381;
const RECOVERY_EFFECTS: &[(u32, u8)] = &[(217200, 1), (259495, 2)];

pub(super) fn command_damage_multiplier(
    world: &World,
    caster: Entity,
    _: Entity,
    spell: &combat::spell_info::SpellInfo,
) -> f32 {
    if spell.id != BEAST_COMMAND {
        return 1.0;
    }
    let Some(pet) = world.get::<combat::hunter_pet::HunterPet>(caster) else {
        return 1.0;
    };
    let owner = pet.owner;
    // SimC76f8118 sc_hunter.cpp2768–2789: the pet command multiplies
    // 1 + Alpha Predator effect2 (+ Pack Mentality while a Howl beast is
    // ready, combined additively) on top of the owner's passive spell mods,
    // which separately apply Alpha Predator effect4.
    let alpha = if has_talent(world, owner, ALPHA_PREDATOR) {
        script_effect_value(world, owner, ALPHA_PREDATOR, 1)
    } else {
        0.0
    };
    let ready = READY
        .iter()
        .any(|ready| combat::auras::has_aura(world, owner, *ready));
    let pack = if ready && has_talent(world, owner, PACK_MENTALITY) {
        script_effect_value(world, owner, PACK_MENTALITY, 0)
    } else {
        0.0
    };
    (100.0 + alpha + pack) / 100.0
}

pub(super) fn recover_cooldowns(world: &mut World, caster: Entity, consumed: u32) {
    if !has_talent(world, caster, PACK_MENTALITY) {
        return;
    }
    // SimC76f8118 hunter3808–3811: both clocks recover per consumed beast,
    // before Incoming Stampede starts. The shared reducer never starts an
    // inactive clock; native472358 effects2/3 supply milliseconds.
    for &(spell, effect) in RECOVERY_EFFECTS {
        let milliseconds = script_effect_value(world, caster, PACK_MENTALITY, effect);
        let seconds = f64::from(milliseconds) * f64::from(consumed) / 1000.0;
        reduce_cooldown(world, caster, spell, seconds);
    }
}
