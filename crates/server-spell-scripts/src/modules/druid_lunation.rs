//! SimC76f8118 sc_druid.cpp1881–1915: foreground Arcane actions earn Lunation CDR.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{script_registry::SpellScriptHook, scripts};
use combat::spell_info::SpellInfo;
use shared::components::ActiveSpec;

const LUNATION: u32 = 429539;
const FURY_OF_ELUNE: u32 = 202770;
const NEW_MOON: u32 = 274281;
const THRASH: u32 = 77758;
const LUNAR_CALLING: u32 = 429523;
const ARCANE_ACTIONS: &[u32] = &[78674, 197626, 8921, 191034, 194153, 197628, 383410, 390414];

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::SuccessfulCastRelationships(
    after_arcane_action,
)];

fn after_arcane_action(world: &mut World, caster: Entity, spell: &SpellInfo) {
    let eligible = ARCANE_ACTIONS.contains(&spell.id)
        || (spell.id == THRASH
            && super::cast_relationships::has_talent(world, caster, LUNAR_CALLING));
    if !eligible
        || !world
            .get::<ActiveSpec>(caster)
            .is_some_and(|spec| spec.0 == 102)
        || !combat::auras::has_aura(world, caster, LUNATION)
    {
        return;
    }
    let fury_seconds =
        -f64::from(scripts::script_effect_value(world, caster, LUNATION, 0)) / 1000.0;
    let moon_seconds =
        -f64::from(scripts::script_effect_value(world, caster, LUNATION, 1)) / 1000.0;
    scripts::reduce_cooldown(world, caster, FURY_OF_ELUNE, fury_seconds);
    // Native New/Half/Full Moon share charge category1727; reduce the pool once.
    scripts::reduce_cooldown(world, caster, NEW_MOON, moon_seconds);
}
