//! Holy Armaments alternate through the native action-bar override aura.
use super::combat;
use bevy::prelude::*;
use combat::auras::{
    AuraApplication, AuraInstance, AuraSource, RemoveReason, scripts::AuraScripts,
};
use combat::class_progression::TraitLoadout;
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

fn advance_divine_guidance(world: &mut World, caster: Entity, spell: u32) {
    if !world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 66)
        || !world.get::<TraitLoadout>(caster).is_some_and(|traits| {
            traits
                .active_talent_spells()
                .any(|talent| talent.spell_id == 433106)
        })
    {
        return;
    }
    match spell {
        // SimC76f8118 sc_paladin.hpp1738–1740: successful Holy Power spenders grant a stack.
        53600 | 85673 => grant_divine_guidance(world, caster),
        // sc_paladin.cpp562–569: a hardcast consumes the accumulated stacks.
        26573 => consume_divine_guidance(world, caster),
        _ => {}
    }
}

fn grant_divine_guidance(world: &mut World, caster: Entity) {
    let mut buff = world
        .resource::<SpellStore>()
        .get(433106)
        .expect("Divine Guidance spell")
        .clone();
    // SimC76f8118 sc_paladin.cpp3718: this source buff has five stacks.
    buff.aura_options.get_or_insert_default().cumulative_aura = 5;
    combat::auras::apply_aura_with_spell(
        world,
        &AuraApplication {
            caster,
            target: caster,
            spell_id: 433106,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
        &buff,
    );
}

fn consume_divine_guidance(world: &mut World, caster: Entity) {
    let stacks = world
        .get::<combat::auras::ActiveAuras>(caster)
        .and_then(|auras| {
            auras.iter().find(|aura| {
                aura.spell_id == 433106 && aura.caster == caster && aura.source == AuraSource::Cast
            })
        })
        .map_or(0, |aura| aura.stacks);
    if stacks == 0 {
        return;
    }
    let store = world.resource::<SpellStore>();
    let talent = store.get(433106).expect("Divine Guidance talent");
    let coefficient = talent
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("Divine Guidance AP coefficient")
        .base_points
        / 10.0
        * f32::from(stacks);
    let target = world
        .get::<combat::networking::CombatTarget>(caster)
        .and_then(|target| target.0);
    if let Some(target) = target {
        cast_guidance_effect(world, caster, target, 433808, coefficient);
    }
    cast_guidance_effect(world, caster, caster, 433807, coefficient * 0.3);
    combat::auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
        aura.spell_id == 433106 && aura.caster == caster && aura.source == AuraSource::Cast
    });
}

fn cast_guidance_effect(
    world: &mut World,
    caster: Entity,
    target: Entity,
    id: u32,
    coefficient: f32,
) {
    let mut heal = world
        .resource::<SpellStore>()
        .get(id)
        .expect("Divine Guidance effect spell")
        .clone();
    heal.effects
        .iter_mut()
        .find(|effect| effect.index == 0)
        .expect("Divine Guidance effect")
        .bonus_coefficient_from_ap = coefficient;
    let amount = combat::spell_cast::effects::scaled_amount(world, caster, &heal, &heal.effects[0]);
    let cast = combat::spell_cast::cast_triggered_with_overrides(
        world,
        caster,
        id,
        Some(target),
        1,
        combat::spell_cast::TriggeredSpellOverrides {
            effect0_amount: Some(amount),
            ..Default::default()
        },
    );
    if !cast {
        warn!("Divine Guidance spell {id} could not be cast");
    }
}

fn extend_zealots_paragon(world: &mut World, caster: Entity, spell: u32) {
    if !matches!(spell, 275779 | 1241413)
        || !world
            .get::<ActiveSpec>(caster)
            .is_some_and(|spec| spec.0 == 66)
        || !world.get::<TraitLoadout>(caster).is_some_and(|traits| {
            traits
                .active_talent_spells()
                .any(|talent| talent.spell_id == 391142)
        })
    {
        return;
    }
    // SimC76f8118 sc_paladin.cpp1414–1429: actual Judgment/HoW casts extend live wings.
    let duration = combat::spell_cast::scripts::script_effect_value(world, caster, 391142, 0);
    combat::auras::extend_owned_duration(world, caster, caster, &[31884, 389539], duration as u32);
}

fn after_cast(world: &mut World, caster: Entity, spell: &SpellInfo) {
    extend_zealots_paragon(world, caster, spell.id);
    advance_shining_light(world, caster, spell.id);
    advance_divine_guidance(world, caster, spell.id);
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
