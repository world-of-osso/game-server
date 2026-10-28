//! Holy Armaments alternate through the native action-bar override aura.
use super::combat;
use bevy::prelude::*;
use combat::auras::{
    AuraApplication, AuraInstance, AuraSource, RemoveReason, scripts::AuraScripts,
};
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_info::{SpellInfo, SpellStore};
use combat::stats::CharacterStats;
use shared::components::{ActiveSpec, Health};

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::SuccessfulCastRelationships(after_cast),
    SpellScriptHook::LateAuraRegistration(register_bulwark),
];

fn register_bulwark(scripts: &mut AuraScripts) {
    let script = scripts.0.entry(432496).or_default();
    script.on_apply = Some(initial_bulwark);
    script.on_periodic = Some(periodic_bulwark);
}

fn initial_bulwark(world: &mut World, target: Entity, aura: &AuraInstance) {
    apply_bulwark_absorb(world, target, aura.caster, 1, true);
}

fn periodic_bulwark(world: &mut World, target: Entity, aura: &AuraInstance, index: u8) {
    if index == 1 {
        apply_bulwark_absorb(world, target, aura.caster, 3, false);
    }
}

fn apply_bulwark_absorb(
    world: &mut World,
    target: Entity,
    caster: Entity,
    index: u8,
    initial: bool,
) {
    // SimC76f8118 sc_paladin.hpp:990–999, 1066–1072: percent/10 of maximum health;
    // versatility applies to the initial grant, not the periodic increment.
    let store = world.resource::<SpellStore>();
    let source = store.get(432496).expect("Holy Bulwark source spell");
    let percent = source
        .effects
        .iter()
        .find(|effect| effect.index == index)
        .expect("Holy Bulwark shield amount")
        .base_points;
    let mut shield = store
        .get(432607)
        .expect("Holy Bulwark absorb spell")
        .clone();
    let health = world
        .get::<Health>(target)
        .expect("living Holy Bulwark target")
        .max;
    let versatility = if initial {
        1.0 + world
            .get::<CharacterStats>(caster)
            .expect("Holy Bulwark caster stats")
            .versatility_pct
            / 100.0
    } else {
        1.0
    };
    shield
        .effects
        .iter_mut()
        .find(|effect| effect.index == 0)
        .expect("Holy Bulwark absorb effect")
        .base_points = health * percent / 1000.0 * versatility;
    combat::auras::apply_aura_with_spell(
        world,
        &AuraApplication {
            caster,
            target,
            spell_id: 432607,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
        &shield,
    );
}

fn advance_shining_light(world: &mut World, caster: Entity, spell: u32) {
    if !world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 66)
    {
        return;
    }
    if spell == 85673 {
        combat::auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
            aura.spell_id == 327510 && aura.caster == caster
        });
        return;
    }
    if spell != 53600 {
        return;
    }
    // SimC76f8118 sc_paladin.cpp2940–2948: every third hardcast grants a free Word of Glory.
    let stacks = world
        .get::<combat::auras::ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == 182104 && aura.caster == caster)
        })
        .map_or(0, |aura| aura.stacks);
    let id = if stacks >= 2 {
        combat::auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
            aura.spell_id == 182104 && aura.caster == caster
        });
        327510
    } else {
        182104
    };
    let mask = if id == 182104 { 3 } else { 1 };
    combat::auras::apply_aura(
        world,
        &AuraApplication {
            caster,
            target: caster,
            spell_id: id,
            effect_mask: mask,
            source: AuraSource::Cast,
        },
    );
}

fn after_cast(world: &mut World, caster: Entity, spell: &SpellInfo) {
    advance_shining_light(world, caster, spell.id);
    // Native70448 spell432478 effect0 is aura332: misc432459 -> amount432472.
    // SimC76f8118 sc_paladin.cpp2748–2752 alternates only on actual hardcasts.
    let Some(override_spell) = world.resource::<SpellStore>().get(432478) else {
        if matches!(spell.id, 432459 | 432472) {
            warn!("Holy Armaments override aura432478 is missing");
        }
        return;
    };
    let Some(effect) = override_spell
        .effects
        .iter()
        .find(|effect| effect.aura == 332)
    else {
        if matches!(spell.id, 432459 | 432472) {
            warn!("Holy Armaments override aura432478 lacks aura332");
        }
        return;
    };
    let override_id = override_spell.id;
    let base = effect.misc_values[0] as u32;
    let replacement = effect.base_points as u32;
    if spell.id == base {
        combat::auras::apply_aura(
            world,
            &AuraApplication {
                caster,
                target: caster,
                spell_id: override_id,
                effect_mask: 1,
                source: AuraSource::Cast,
            },
        );
    } else if spell.id == replacement {
        combat::auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
            aura.spell_id == override_id && aura.caster == caster
        });
    }
}
