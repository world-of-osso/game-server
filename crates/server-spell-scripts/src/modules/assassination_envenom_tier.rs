//! The MID2 two-piece fills Envenom's native damage-buff effects.
use super::combat;
use bevy::prelude::*;
use combat::auras::AuraInstance;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::PreparePolicySpell(prepare_inspiring_strike),
    SpellScriptHook::AfterAuraApplication(apply_tier_bonus),
];

// SimC76f8118 sc_rogue.cpp4357–4375/11141–11152: refreshing active
// Envenom earns Inspiring Strike on its native direct/periodic/auto masks.
fn prepare_inspiring_strike(world: &World, caster: Entity, spell: &mut SpellInfo) {
    if spell.id != 32645
        || !combat::auras::has_aura(world, caster, 32645)
        || !combat::auras::has_aura(world, caster, 1250036)
    {
        return;
    }
    let value = combat::spell_cast::scripts::script_effect_value(world, caster, 1250036, 0);
    let tier = if combat::auras::has_aura(world, caster, 1296590) {
        combat::spell_cast::scripts::script_effect_value(world, caster, 1296590, 1)
    } else {
        0.0
    };
    // SimC treats tier and Inspiring Strike as separate multiplicative buffs.
    for effect in spell.effects.iter_mut().filter(|effect| matches!(effect.index, 7..=9)) {
        effect.base_points += value * (1.0 + tier / 100.0);
    }
}

fn apply_tier_bonus(world: &mut World, unit: Entity, aura: &AuraInstance) {
    if aura.spell_id != 32645
        || aura.caster != unit
        || !combat::auras::has_aura(world, unit, 1296590)
    {
        return;
    }
    let amount = combat::spell_cast::scripts::script_effect_value(world, unit, 1296590, 1);
    // SimC76f8118 sc_rogue.cpp11249–11259: the two-piece value fills
    // Envenom's native direct, periodic and auto-attack damage effects.
    for index in [7, 8, 9] {
        let native = aura
            .effects
            .iter()
            .find(|effect| effect.index == index)
            .expect("Envenom retains native tier effect")
            .amount;
        assert!(combat::auras::set_effect_amount(
            world,
            unit,
            aura.instance_id,
            index,
            native + amount
        ));
    }
}
