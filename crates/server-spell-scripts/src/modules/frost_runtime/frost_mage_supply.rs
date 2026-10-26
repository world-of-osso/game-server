//! Frost and Spellslinger execution producers, using authored native amounts.
use super::{AfterCastHook, combat, script_effect_value};
use combat::auras::{self, ActiveAuras, AuraApplication, AuraSource, procs};
use combat::spell_info::SpellInfo;
use bevy::prelude::*;
use shared::components::ActiveSpec;

const AUGURY: u32 = 1280165;
// SimC76f8118 sc_mage.cpp371/6638–6641: pinned default PRD cap.
const AUGURY_CAP: u32 = 21;
#[derive(Component, Default)]
struct AuguryAttempts(u32);

pub(super) fn owns_proc(spell: u32) -> bool {
    spell == AUGURY
}

fn frost(world: &World, caster: Entity) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 64)
}

fn roll_talent(world: &World, caster: Entity, talent: u32, effect: u8) -> bool {
    auras::has_aura(world, caster, talent)
        && combat::random::random::<f32>() * 100.0
            < script_effect_value(world, caster, talent, effect)
}

fn apply_buff(world: &mut World, caster: Entity, spell_id: u32) {
    auras::apply_aura(
        world,
        &AuraApplication {
            caster,
            target: caster,
            spell_id,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
}

pub(super) fn after_cast_hook(spell: u32) -> Option<AfterCastHook> {
    matches!(spell, 116 | 199786 | 84714).then_some(after_cast)
}

fn after_cast(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !frost(world, caster) {
        return;
    }
    // sc_mage.cpp4085–4086/4281–4283: preserve each producer's RNG order.
    match spell.id {
        116 => {
            roll_fingers(world, caster, 112965);
            roll_brain_freeze(world, caster, spell);
        }
        199786 => {
            roll_brain_freeze(world, caster, spell);
            roll_fingers(world, caster, 379993);
            roll_fingers(world, caster, 112965);
        }
        84714 => schedule_orb_splinters(world, caster),
        _ => {}
    }
}

fn roll_fingers(world: &mut World, caster: Entity, talent: u32) {
    if roll_talent(world, caster, talent, 0) {
        apply_buff(world, caster, super::frost_mage_procs::FINGERS);
    }
}

fn roll_brain_freeze(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !roll_talent(world, caster, 190447, 0) {
        return;
    }
    super::frost_mage_procs::grant_random_brain_freeze(world, caster);
    after_brain_freeze(world, caster, spell.cast_primary_target);
}

pub(super) fn after_brain_freeze(world: &mut World, caster: Entity, primary: Option<Entity>) {
    // sc_mage.cpp7436–7437: these follow the successful roll immediately,
    // even when the existing Brain Freeze buff refresh is deferred150ms.
    if auras::has_aura(world, caster, 461261) {
        apply_buff(world, caster, 461531);
    }
    if auras::has_aura(world, caster, 444675) {
        let count = script_effect_value(world, caster, 444675, 0) as u32;
        if let Some(target) = primary {
            super::conjure_count(world, caster, target, count);
        }
    }
}

fn schedule_orb_splinters(world: &mut World, caster: Entity) {
    if !auras::has_aura(world, caster, 444256) {
        return;
    }
    // sc_mage.cpp4181–4209: activation minimum500ms, then one conjure per
    //500ms pulse for native Splintering Orbs effect6's count.
    let count = script_effect_value(world, caster, 444256, 5) as u32;
    for index in 0..count {
        super::frost_mage_events::schedule_splinters(world, caster, 1, 500 + index * 500);
    }
}

pub(super) fn after_launch(world: &mut World, caster: Entity, spell: &SpellInfo) {
    let spec = world.get::<ActiveSpec>(caster).map(|spec| spec.0);
    let eligible = matches!((spec, spell.id), (Some(62), 443763) | (Some(64), 443722));
    if !eligible {
        return;
    }
    let instance = world.get::<ActiveAuras>(caster).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.caster == caster && aura.spell_id == AUGURY)
            .map(|aura| aura.instance_id)
    });
    let Some(instance) = instance else {
        return;
    };
    // sc_mage.cpp5419–5423: start the native ICD on every admitted attempt,
    // including failed PRD rolls, not only successful conjures.
    if procs::start_policy_proc(world, caster, instance).is_none() {
        return;
    }
    if !roll_augury(world, caster) {
        return;
    }
    let count = script_effect_value(world, caster, AUGURY, 1) as u32;
    super::frost_mage_events::schedule_splinters(world, caster, count, 100);
}

fn roll_augury(world: &mut World, caster: Entity) -> bool {
    let probability = f64::from(script_effect_value(world, caster, AUGURY, 0)) / 100.0;
    let constant = combat::random::capped_accumulated_proc_constant(probability, AUGURY_CAP);
    let attempts = world
        .entity_mut(caster)
        .entry::<AuguryAttempts>()
        .or_default()
        .into_mut()
        .0
        + 1;
    let chance = if attempts >= AUGURY_CAP {
        1.0
    } else {
        constant * f64::from(attempts)
    };
    let success = combat::random::random::<f64>() < chance;
    world.get_mut::<AuguryAttempts>(caster).unwrap().0 = if success { 0 } else { attempts };
    success
}
