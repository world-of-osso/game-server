//! 70391 primary-target admission and maximum-health damage; talents remain separate.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::script_registry::{DamageAmountHook, SpellScriptHook};
use combat::spell_cast::scripts::script_effect_value;
use shared::components::{Health, Npc};

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::DamageAmount(damage_hook),
    SpellScriptHook::CheckTarget(check_target),
];

fn damage_hook(spell: u32) -> Option<DamageAmountHook> {
    (spell == TOUCH_OF_DEATH).then_some(damage)
}

fn check_target(
    world: &World,
    caster: Entity,
    spell: u32,
    target: Entity,
) -> Result<(), combat::spell_cast::CastFailure> {
    if spell == TOUCH_OF_DEATH && !target_allowed(world, caster, target) {
        return Err(combat::spell_cast::CastFailure::InvalidTarget);
    }
    Ok(())
}

pub(super) const TOUCH_OF_DEATH: u32 = 322109;
const IMPROVED_TOUCH_OF_DEATH: u32 = 322113;

/// 70391 root s4/s3 supply 100%/35%; SimC76f8118 sc_monk.cpp2701–2711.
pub(super) fn damage(world: &World, caster: Entity, target: Entity, _amount: f32) -> f32 {
    let monk = world.get::<Health>(caster).expect("living Monk health");
    let victim = world.get::<Health>(target).expect("living target health");
    let weaker_creature = world.get::<Npc>(target).is_some() && victim.current < monk.max;
    let index = if weaker_creature { 3 } else { 2 };
    monk.max * script_effect_value(world, caster, TOUCH_OF_DEATH, index) / 100.0
}

/// 70391 descriptions require a weaker creature, or the learned improved rank
/// below its authored percentage. SimC76f8118 sc_monk.cpp2685 compares the
/// creature's current health to the Monk's maximum; strict boundaries follow DB2.
pub(super) fn target_allowed(world: &World, caster: Entity, target: Entity) -> bool {
    let monk = world.get::<Health>(caster).expect("living Monk health");
    let victim = world.get::<Health>(target).expect("living target health");
    let weaker_creature = world.get::<Npc>(target).is_some() && victim.current < monk.max;
    if weaker_creature {
        return true;
    }
    if !combat::auras::has_aura(world, caster, IMPROVED_TOUCH_OF_DEATH) {
        return false;
    }
    let threshold = script_effect_value(world, caster, IMPROVED_TOUCH_OF_DEATH, 0);
    victim.current * 100.0 < victim.max * threshold
}
