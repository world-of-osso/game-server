//! Authored cast-result relationships; native spell effects supply proc values.
use bevy::prelude::*;
use shared::components::ActiveSpec;

use crate::spell_cast;
use crate::spell_info::{SpellInfo, SpellStore};

struct CastReset {
    spec: u32,
    producer: u32,
    chance_effect: u8,
    actions: &'static [u32],
}

const RESETS: &[CastReset] = &[
    // SimC76f8118 sc_paladin_retribution.cpp720–733: Final Verdict rolls
    // effect2 after execution, then resets Judgment AND Hammer of Wrath.
    // cooldown.hpp63/cooldown.cpp370–379: each reset(true) restores one charge.
    CastReset {
        spec: 70,
        producer: 383328,
        chance_effect: 1,
        actions: &[20271, 24275],
    },
];

pub(super) fn hook(spell: u32) -> Option<super::AfterCastHook> {
    RESETS
        .iter()
        .any(|reset| reset.producer == spell)
        .then_some(roll_resets)
}

fn roll_resets(world: &mut World, caster: Entity, spell: &SpellInfo) {
    for reset in RESETS.iter().filter(|reset| reset.producer == spell.id) {
        if world
            .get::<ActiveSpec>(caster)
            .is_none_or(|spec| spec.0 != reset.spec)
        {
            continue;
        }
        let chance = super::script_effect_value(world, caster, spell.id, reset.chance_effect);
        if crate::random::random::<f32>() * 100.0 >= chance {
            continue;
        }
        for action in reset.actions {
            restore_action_charge(world, caster, *action);
        }
    }
}

fn restore_action_charge(world: &mut World, caster: Entity, action: u32) {
    let category = world
        .resource::<SpellStore>()
        .get(action)
        .and_then(|spell| spell.categories.as_ref())
        .expect("native reset action retains its charge category")
        .charge_category;
    spell_cast::restore_charges(world, caster, category, 1);
}
