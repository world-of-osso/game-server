//! The MID2 two-piece fills Envenom's native damage-buff effects.
use super::combat;
use bevy::prelude::*;
use combat::auras::AuraInstance;
use combat::spell_cast::script_registry::SpellScriptHook;

pub(super) const HOOKS: &[SpellScriptHook] =
    &[SpellScriptHook::AfterAuraApplication(apply_tier_bonus)];

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
