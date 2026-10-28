//! Shared Hellcaller Wither counter semantics, not a damage multiplier.
use super::combat::spell_cast::script_registry::SpellScriptHook;
use bevy::prelude::*;
use shared::components::ActiveSpec;

// SimC76f8118 sc_warlock_actions.cpp1066 ignores counter stacks for tick
// damage;1174–1186 cancels refresh's automatic+1 without discarding earned
// stacks. The original native coefficient, phase and lifetime remain unchanged.
// actions273–275 also reads Wither's target effect without counter stacks;
// Ashen Remains' native incoming modifier applies once, not once per counter.
pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::CounterOnlyDotStacks(&[445474]),
    SpellScriptHook::NonStackingIncomingAuras(&[445474]),
    SpellScriptHook::EffectEnabled(xavius_effect_enabled),
];

// SimC76f8118 sc_warlock_init.cpp82–83: Affliction registers only effects
// 1 and 3, while Destruction registers every effect except effect 1.
fn xavius_effect_enabled(world: &World, caster: Entity, spell: u32, index: u8) -> bool {
    if spell != 440046 {
        return true;
    }
    match world.get::<ActiveSpec>(caster).map(|spec| spec.0) {
        Some(265) => matches!(index, 0 | 2),
        Some(267) => index != 0,
        _ => true,
    }
}
