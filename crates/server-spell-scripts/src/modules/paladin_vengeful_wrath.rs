//! Vengeful Wrath scales Hammer of Wrath against the target's remaining health.
use bevy::prelude::*;
use shared::components::Health;
use super::combat;
use combat::spell_cast::{script_registry::SpellScriptHook, scripts::script_effect_value};
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::TargetDamageMultiplier(multiplier)];

fn multiplier(world: &World, caster: Entity, target: Entity, spell: &SpellInfo) -> f32 {
    const TALENT: u32 = 1241958;
    if !matches!(spell.id, 24275 | 1241413 | 1279408) || !combat::auras::has_aura(world, caster, TALENT) {
        return 1.0;
    }
    // SimC76f8118 sc_paladin.cpp1788–1795: the native selected percentage
    // rises linearly from zero at full health to its maximum at 20% health.
    let health = world.get::<Health>(target).expect("Hammer of Wrath target health");
    let fraction = (health.current / health.max).clamp(0.0, 1.0);
    let progress = 1.0 - ((fraction - 0.2).max(0.0) / 0.8);
    1.0 + script_effect_value(world, caster, TALENT, 0) / 100.0 * progress
}
