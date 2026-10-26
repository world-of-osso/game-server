//! Selected Cruelty and Wrath and Fury modifiers follow the live Enrage aura.
use bevy::prelude::*;

use super::combat;
use combat::auras::{ActiveAuras, AuraInstance, RemoveReason};
use combat::spell_cast::script_registry::SpellScriptHook;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::EarlyAuraRegistration(register)];

const ENRAGE: u32 = 184362;
const MODIFIERS: [(u32, u8); 2] = [(392931, 0), (392936, 1)];

fn register(scripts: &mut combat::auras::scripts::AuraScripts) {
    let enrage = scripts.0.entry(ENRAGE).or_default();
    enrage.on_apply = Some(activate);
    enrage.on_remove = Some(deactivate);
    for (spell, _) in MODIFIERS {
        scripts.0.entry(spell).or_default().on_apply = Some(bind_passive);
    }
}

fn bind_passive(world: &mut World, unit: Entity, aura: &AuraInstance) {
    if aura.caster != unit {
        return;
    }
    let active = combat::auras::has_aura(world, unit, ENRAGE);
    update_modifier(world, unit, aura.spell_id, aura.instance_id, active);
}

fn activate(world: &mut World, unit: Entity, aura: &AuraInstance) {
    if aura.caster == unit {
        update_passives(world, unit, true);
    }
}

fn deactivate(world: &mut World, unit: Entity, aura: &AuraInstance, _: RemoveReason) {
    if aura.caster == unit {
        update_passives(world, unit, false);
    }
}

fn update_passives(world: &mut World, unit: Entity, active: bool) {
    let instances: Vec<_> = world
        .get::<ActiveAuras>(unit)
        .into_iter()
        .flat_map(|auras| auras.iter())
        .filter(|aura| {
            aura.caster == unit && MODIFIERS.iter().any(|(spell, _)| *spell == aura.spell_id)
        })
        .map(|aura| (aura.spell_id, aura.instance_id))
        .collect();
    for (spell, instance) in instances {
        update_modifier(world, unit, spell, instance, active);
    }
}

fn update_modifier(world: &mut World, unit: Entity, spell: u32, instance: u32, active: bool) {
    // SimC76f8118 sc_warrior.cpp1106/4939–4947/5099–5113/7834–7835:
    // both selected rank modifiers apply only while Enrage is active.
    let index = MODIFIERS
        .iter()
        .find(|(id, _)| *id == spell)
        .expect("registered Fury modifier")
        .1;
    let amount = if active {
        combat::spell_cast::scripts::script_effect_value(world, unit, spell, index)
    } else {
        0.0
    };
    combat::auras::set_effect_amount(world, unit, instance, index, amount);
}
