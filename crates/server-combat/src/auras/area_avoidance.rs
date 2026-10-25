//! TC a352b1fa: Spell::DoEffectOnLaunchTarget, periodic damage/leech ticks,
//! Unit::CalculateAOEAvoidance. Classification belongs to the damage effect,
//! not every effect of a spell merely because one sibling selects an area.
use super::{ActiveAuras, handlers};
use crate::spell_info::{SpellEffect, SpellInfo};
use bevy::prelude::*;

const ATTR4_IGNORE_DAMAGE_TAKEN_MODIFIERS: u32 = 0x100;
const ATTR5_TREAT_AS_AREA_EFFECT: u32 = 0x8000;
const ATTR7_TREAT_AS_NPC_AOE: u32 = 0x0400_0000;

fn selects_area(effect: &SpellEffect) -> bool {
    // Pinned SpellImplicitTargetInfo::IsArea: AREA or CONE, not LINE.
    const AREA_TARGETS: &[u32] = &[
        7, 8, 15, 16, 20, 24, 30, 31, 33, 34, 37, 51, 52, 54, 56, 59, 60, 61, 93, 104, 105, 108,
        109, 110, 115, 116, 118, 119, 120, 122, 123, 128, 129, 130, 136,
    ];
    effect
        .implicit_targets
        .iter()
        .any(|id| AREA_TARGETS.contains(id))
        || matches!(
            effect.effect,
            27 | 35 | 65 | 119 | 128 | 129 | 143 | 202 | 271
        )
}

fn is_area_damage(spell: &SpellInfo, effect: Option<&SpellEffect>, periodic: bool) -> bool {
    let attributes = spell.misc.as_ref().map_or([0; 17], |misc| misc.attributes);
    if periodic && attributes[4] & ATTR4_IGNORE_DAMAGE_TAKEN_MODIFIERS != 0 {
        return false;
    }
    effect.is_some_and(selects_area)
        || attributes[5] & ATTR5_TREAT_AS_AREA_EFFECT != 0
        || attributes[7] & ATTR7_TREAT_AS_NPC_AOE != 0
}

pub(crate) fn reduce_damage(
    world: &World,
    target: Entity,
    spell: &SpellInfo,
    effect: Option<&SpellEffect>,
    periodic: bool,
    damage: f32,
) -> f32 {
    if !is_area_damage(spell, effect, periodic) {
        return damage;
    }
    let factor = world.get::<ActiveAuras>(target).map_or(1.0, |auras| {
        auras
            .iter()
            .flat_map(|aura| {
                let stacks = aura.stack_multiplier();
                aura.effects
                    .iter()
                    .filter(|effect| {
                        effect.aura_type == handlers::MOD_AOE_DAMAGE_AVOIDANCE
                            && effect.misc_values[0] as u32 & spell.school_mask() != 0
                    })
                    .map(move |effect| 1.0 + effect.amount * stacks / 100.0)
            })
            .product::<f32>()
    });
    (damage * factor).max(0.0).trunc()
}
