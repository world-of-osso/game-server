//! Pinned Frost proc producers; native spells supply their effects and amounts.
use bevy::prelude::*;
use shared::components::{ActiveSpec, WeaponType};

use crate::{auras, random, spell_cast, spell_info::SpellStore, stats::CharacterStats};

const KILLING_MACHINE: u32 = 51128;
const RIME: u32 = 59057;
const OBLITERATION: u32 = 281238;
const PILLAR: u32 = 51271;

#[derive(Component, Default)]
struct KillingMachineAttempts(u32);

pub(super) fn owns_proc(spell: u32) -> bool {
    matches!(spell, KILLING_MACHINE | RIME | OBLITERATION)
}

fn is_frost(world: &World, caster: Entity) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 251)
}

fn trigger_native_buff(world: &mut World, caster: Entity, source: u32) {
    let buff = world
        .resource::<SpellStore>()
        .get(source)
        .and_then(|spell| spell.effects.iter().find(|effect| effect.index == 0))
        .expect("active Frost passive retains native effect0")
        .trigger_spell;
    if !spell_cast::try_cast_triggered(world, caster, buff, Some(caster), 1) {
        warn!(?caster, source, buff, "Native Frost proc buff refused");
    }
}

pub(super) fn after_white_hit(world: &mut World, caster: Entity, critical: bool) {
    if !critical || !is_frost(world, caster) || !auras::has_aura(world, caster, KILLING_MACHINE) {
        return;
    }
    let Some(stats) = world.get::<CharacterStats>(caster) else {
        return;
    };
    // SimC76f8118 sc_death_knight.cpp7716–7719/14824–14825: critical
    // white swings use accumulated PRD,0.3 DW or1.0 with Might and a2H.
    let two_hand = stats.main_hand.kind == WeaponType::TwoHand;
    let probability = if two_hand && auras::has_aura(world, caster, 81333) {
        1.0
    } else {
        0.3
    };
    let attempts = world
        .get::<KillingMachineAttempts>(caster)
        .map_or(1, |state| state.0 + 1);
    world
        .entity_mut(caster)
        .insert(KillingMachineAttempts(attempts));
    let chance = random::accumulated_proc_constant(probability) * f64::from(attempts);
    if random::random::<f64>() >= chance {
        return;
    }
    world.get_mut::<KillingMachineAttempts>(caster).unwrap().0 = 0;
    trigger_native_buff(world, caster, KILLING_MACHINE);
}

pub(super) fn after_launch(world: &mut World, caster: Entity, spell: u32, foreground: bool) {
    if !foreground || !is_frost(world, caster) {
        return;
    }
    // SimC76f8118 sc_death_knight.cpp10889–10900/11177–11189:
    // FS/GA roll Rime's native effect2 (not its displayed effect1).
    if matches!(spell, 49143 | 194913) && auras::has_aura(world, caster, RIME) {
        let chance = super::script_effect_value(world, caster, RIME, 1);
        if random::random::<f32>() * 100.0 < chance {
            trigger_native_buff(world, caster, RIME);
        }
    }
    if matches!(spell, 49143 | 194913 | 49184) {
        trigger_obliteration(world, caster);
    }
}

fn trigger_obliteration(world: &mut World, caster: Entity) {
    // sc_death_knight.cpp10889–10897/11177–11186/11614–11622:
    // actual FS/GA/HB during Pillar grants KM and rolls the native rune gain.
    if !auras::has_aura(world, caster, OBLITERATION) || !auras::has_aura(world, caster, PILLAR) {
        return;
    }
    if auras::has_aura(world, caster, KILLING_MACHINE) {
        trigger_native_buff(world, caster, KILLING_MACHINE);
    }
    let chance = super::script_effect_value(world, caster, OBLITERATION, 1);
    if random::random::<f32>() * 100.0 >= chance {
        return;
    }
    if !spell_cast::try_cast_triggered(world, caster, 281327, Some(caster), 1) {
        warn!(?caster, "Native Obliteration rune grant refused");
    }
}
