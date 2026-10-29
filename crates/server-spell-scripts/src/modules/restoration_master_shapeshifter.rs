//! Master Shapeshifter mana children retain their native level-scaled amounts.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{
    self, CostPool, script_registry::SpellScriptHook, scripts::AfterCastHook,
};
use combat::spell_info::SpellInfo;
use shared::components::PowerType;

const TALENT: u32 = 289_237;
const SPELL_MANA: u32 = 411_146;
const BEAR_MANA: u32 = 411_144;
const CAT_MANA: u32 = 411_143;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::NativeAfterCast(after_cast),
    SpellScriptHook::AfterSpellPayment(cat_payment),
];

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    matches!(spell, 5_176 | 197_626 | 197_628 | 192_081).then_some(grant_mana)
}

fn grant_mana(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !combat::auras::has_aura(world, caster, TALENT) {
        return;
    }
    let child = if spell.id == 192_081 {
        BEAR_MANA
    } else {
        SPELL_MANA
    };
    spell_cast::cast_triggered(world, caster, child, Some(caster), 1);
}

fn cat_payment(world: &mut World, caster: Entity, spell: &SpellInfo, costs: &[(CostPool, f32)]) {
    if !matches!(spell.id, 1_079 | 22_568 | 22_570)
        || !combat::auras::has_aura(world, caster, TALENT)
    {
        return;
    }
    let required = spell_cast::scripts::script_effect_value(world, caster, TALENT, 3);
    if spell_cast::spent(costs, PowerType::ComboPoints) >= required {
        spell_cast::cast_triggered(world, caster, CAT_MANA, Some(caster), 1);
    }
}
