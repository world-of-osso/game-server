//! Master Shapeshifter mana children retain their native level-scaled amounts.
use super::combat;
use bevy::prelude::*;
use combat::auras::{AuraInstance, procs::AuraEvent, scripts::AuraScripts};
use combat::spell_cast::{
    self, CostPool, script_registry::SpellScriptHook, scripts::AfterCastHook,
};
use combat::spell_info::SpellInfo;
use shared::components::PowerType;

const TALENT: u32 = 289_237;
const SPELL_MANA: u32 = 411_146;
const CAT_MANA: u32 = 411_143;
const IRONFUR: u32 = 192_081;
const BEAR_FORM: u32 = 5_487;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::NativeAfterCast(after_cast),
    SpellScriptHook::AfterSpellPayment(cat_payment),
    SpellScriptHook::LateAuraRegistration(register),
];

fn register(scripts: &mut AuraScripts) {
    scripts.0.entry(TALENT).or_default().check_proc = Some(bear_ironfur_only);
}

// Effect 4 triggers 411144 with a zero class mask on every successful cast;
// TrinityCore ea5113c has no script for it. The 70448 tooltip limits that
// mana to Ironfur in Bear Form.
fn bear_ironfur_only(world: &World, aura: &AuraInstance, event: &AuraEvent) -> bool {
    event.actor == aura.caster
        && event.spell_id == Some(IRONFUR)
        && combat::auras::has_aura(world, aura.caster, BEAR_FORM)
}

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    matches!(spell, 5_176 | 197_626 | 197_628).then_some(grant_mana)
}

fn grant_mana(world: &mut World, caster: Entity, _: &SpellInfo) {
    if combat::auras::has_aura(world, caster, TALENT) {
        spell_cast::cast_triggered(world, caster, SPELL_MANA, Some(caster), 1);
    }
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
