//! Unnatural Causes uses the native target-damage eligibility metadata.
use bevy::prelude::*;
use shared::components::Health;

use super::combat;
use combat::auras::ActiveAuras;
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::{SpellEffect, SpellInfo, SpellStore};

const TALENT: u32 = 459527;
const ELIGIBILITY: u32 = 459529;
const DAMAGE_FROM_CASTER: u32 = 271;
const DAMAGE_FROM_CASTER_LABEL: u32 = 537;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::TargetDamageMultiplier(multiplier)];

fn eligible(effect: &SpellEffect, spell: &SpellInfo) -> bool {
    match effect.aura {
        DAMAGE_FROM_CASTER => spell.class_options.as_ref().is_some_and(|class| {
            effect
                .spell_class_mask
                .iter()
                .zip(class.spell_class_mask)
                .any(|(left, right)| *left & right != 0)
        }),
        DAMAGE_FROM_CASTER_LABEL => u32::try_from(effect.misc_values[0])
            .is_ok_and(|label| spell.labels.binary_search(&label).is_ok()),
        _ => false,
    }
}

fn multiplier(world: &World, caster: Entity, target: Entity, spell: &SpellInfo) -> f32 {
    // SimC76f8118 sc_hunter.cpp1482–1509/2640–2664 uses the native
    // Unnatural Causes target masks; bugs=true excludes Volley, direct Wildfire
    // and Laceration (cpp6934–6938/7056–7059/5161–5164).
    if matches!(spell.id, 260247 | 265157 | 459560) {
        return 1.0;
    }
    let owner = world
        .get::<combat::hunter_pet::HunterPet>(caster)
        .map_or(caster, |pet| pet.owner);
    let selected = world.get::<ActiveAuras>(owner).is_some_and(|auras| {
        auras
            .iter()
            .any(|aura| aura.caster == owner && aura.spell_id == TALENT)
    });
    if !selected {
        return 1.0;
    }
    let source = world
        .resource::<SpellStore>()
        .get(ELIGIBILITY)
        .expect("native Unnatural Causes eligibility");
    let same_family = source
        .class_options
        .as_ref()
        .zip(spell.class_options.as_ref())
        .is_some_and(|(source, action)| source.spell_class_set == action.spell_class_set);
    if !same_family || !source.effects.iter().any(|effect| eligible(effect, spell)) {
        return 1.0;
    }
    let percent = spell_cast::scripts::script_effect_value(world, owner, TALENT, 0);
    let threshold = spell_cast::scripts::script_effect_value(world, owner, TALENT, 2);
    let health = world
        .get::<Health>(target)
        .expect("damage recipient has health");
    let execute = health.max > 0.0 && 100.0 * health.current / health.max < threshold;
    let bonus = if execute {
        let increase = spell_cast::scripts::script_effect_value(world, owner, TALENT, 1);
        percent * (1.0 + increase / 100.0)
    } else {
        percent
    };
    1.0 + bonus / 100.0
}
