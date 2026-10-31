//! SimC76f8118 sc_shaman.cpp2832–2852: Elemental Unity's periodic factors
//! apply even when the corresponding elemental is absent.
use bevy::prelude::*;

use crate::auras::{self, ActiveAuras, spell_mods};
use crate::spell_info::{SpellInfo, SpellStore};

const UNITY: u32 = 462866;
const PRIMAL_ELEMENTALIST: u32 = 117013;
const SOURCES: &[(u32, u8, &[u32])] = &[
    (188592, 4, &[188592, 118291]),
    (157299, 3, &[157299, 157319]),
];

pub(super) fn periodic_multiplier(world: &World, caster: Entity, spell: &SpellInfo) -> f32 {
    if !world
        .get::<crate::class_progression::ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 262)
        || !auras::has_aura(world, caster, UNITY)
    {
        return 1.0;
    }
    let primal = auras::has_aura(world, caster, PRIMAL_ELEMENTALIST);
    SOURCES
        .iter()
        .filter(|(root, index, active)| {
            affects_spell(world, *root, *index, spell) && !has_elemental_aura(world, caster, active)
        })
        .map(|(root, _, _)| {
            let percent = if primal {
                super::script_effect_value(world, caster, UNITY, 1)
            } else {
                super::script_effect_value(world, caster, *root, 3)
            };
            1.0 + percent / 100.0
        })
        .product()
}

fn affects_spell(world: &World, root: u32, index: u8, spell: &SpellInfo) -> bool {
    let source = world
        .resource::<SpellStore>()
        .get(root)
        .expect("native elemental spell");
    let class = source
        .class_options
        .as_ref()
        .expect("native elemental class options");
    let effect = source
        .effects
        .iter()
        .find(|effect| effect.index == index)
        .expect("native elemental modifier effect");
    spell_mods::class_mask_matches(class.spell_class_set, &effect.spell_class_mask, spell)
}

fn has_elemental_aura(world: &World, caster: Entity, ids: &[u32]) -> bool {
    world.get::<ActiveAuras>(caster).is_some_and(|auras| {
        auras
            .iter()
            .any(|aura| aura.caster == caster && ids.contains(&aura.spell_id))
    })
}
