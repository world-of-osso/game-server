//! SimC76f8118 sc_monk.cpp555–647, 2609–2611 and 4101–4108.
use bevy::prelude::*;
use shared::components::ActiveSpec;

use super::combat;
use combat::auras::{self, AuraApplication, AuraSource, RemoveReason};
use combat::class_progression::TraitLoadout;
use combat::spell_cast::{
    self,
    script_registry::{AreaFilterLookup, AreaTargetFilter, SpellScriptHook},
    scripts::{AfterCastHook, script_effect_value},
};
use combat::spell_info::SpellInfo;

const TALENT: u32 = 1270958;
const POTENTIAL: u32 = 1271048;
const BUFF: u32 = 1270990;
const DAMAGE: u32 = 1271011;
const HEAL: u32 = 1271045;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterResourceValues(cast_hook),
    SpellScriptHook::AreaFilter(area_filter as AreaFilterLookup),
];

fn selected(world: &World, actor: Entity, talent: u32) -> bool {
    world.get::<TraitLoadout>(actor).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|entry| entry.spell_id == talent)
    })
}

fn cast_hook(spell: u32) -> Option<AfterCastHook> {
    match spell {
        121253 | 1241059 | 322507 => Some(grant),
        100780 => Some(spend),
        _ => None,
    }
}

fn grant(world: &mut World, actor: Entity, spell: &SpellInfo) {
    if !world
        .get::<ActiveSpec>(actor)
        .is_some_and(|spec| spec.0 == 268)
        || !selected(world, actor, TALENT)
    {
        return;
    }
    let count = if spell.id == 121253 {
        usize::from(selected(world, actor, POTENTIAL))
    } else {
        script_effect_value(world, actor, TALENT, 4) as usize
    };
    for _ in 0..count {
        auras::apply_aura(
            world,
            &AuraApplication {
                caster: actor,
                target: actor,
                spell_id: BUFF,
                effect_mask: u32::MAX,
                source: AuraSource::Cast,
            },
        );
    }
}

fn spend(world: &mut World, actor: Entity, spell: &SpellInfo) {
    let Some(target) = spell.cast_primary_target else {
        return;
    };
    let stacks = world
        .get::<auras::ActiveAuras>(actor)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == BUFF && aura.caster == actor)
        })
        .map_or(0, |aura| aura.stacks);
    if stacks == 0 || !selected(world, actor, TALENT) {
        return;
    }
    let stack_multiplier = 1.0 + f32::from(stacks);
    let damage_factor = script_effect_value(world, actor, TALENT, 0) / 100.0 * stack_multiplier;
    let healing_factor = script_effect_value(world, actor, TALENT, 2) / 100.0 * stack_multiplier;
    launch_surge(world, actor, target, DAMAGE, damage_factor);
    launch_surge(world, actor, actor, HEAL, healing_factor);
    auras::remove_auras(world, actor, RemoveReason::Consumed, |aura| {
        aura.spell_id == BUFF && aura.caster == actor
    });
}

fn launch_surge(world: &mut World, actor: Entity, target: Entity, id: u32, factor: f32) {
    let Some(mut spell) = spell_cast::load_triggered_spell(world, id, 1) else {
        return;
    };
    for effect in &mut spell.effects {
        effect.bonus_coefficient *= factor;
    }
    spell_cast::launch_triggered_spell(world, actor, &spell, Some(target), 1);
}

fn area_filter(spell: u32, index: u8) -> Option<AreaTargetFilter> {
    (spell == DAMAGE && index == 1).then_some(exclude_primary)
}

fn exclude_primary(_: &World, spell: &SpellInfo, _: Option<Entity>, targets: &mut Vec<Entity>) {
    targets.retain(|target| Some(*target) != spell.cast_primary_target);
}
