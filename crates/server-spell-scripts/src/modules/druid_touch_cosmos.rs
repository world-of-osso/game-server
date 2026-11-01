//! SimC76f8118 sc_druid.cpp:6859–6862, 6980–6983, 8503, 9053.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, RemoveReason};
use combat::class_progression::KnownSpells;
use combat::spell_cast::{self, script_registry::SpellScriptHook, scripts::script_effect_value};
use combat::spell_info::SpellInfo;

const TALENT: u32 = 450356;
const BUFF: u32 = 450360;

pub(super) const HOOKS: &[SpellScriptHook] =
    &[SpellScriptHook::SuccessfulCastRelationships(after_cast)];

fn after_cast(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !world
        .get::<KnownSpells>(caster)
        .is_some_and(|known| known.contains(TALENT))
    {
        return;
    }
    let index = match spell.id {
        190984 => 0,
        194153 => 1,
        78674 | 191034 => {
            auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
                aura.spell_id == BUFF && aura.caster == caster
            });
            return;
        }
        _ => return,
    };
    let chance = script_effect_value(world, caster, TALENT, index);
    if combat::random::random::<f32>() * 100.0 < chance {
        spell_cast::cast_triggered(world, caster, BUFF, Some(caster), 1);
    }
}
