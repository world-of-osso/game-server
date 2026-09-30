//! Spell scripts (TrinityCore `SpellScript`) keyed by spell ID: behaviour that the
//! spell's DB2 effects do not carry.

use bevy::prelude::*;

/// Frostbolt: its effects are TRIGGER_MISSILE 32 to 228597 (SCHOOL_DAMAGE) only.
const FROSTBOLT: u32 = 116;
/// Chilled: APPLY_AURA MOD_DECREASE_SPEED 33 (-50%, 8 s).
const CHILLED: u32 = 205_708;

/// `SpellHitFn`: runs once per unit the spell lands on, after its effects resolved there.
pub type HitHook = fn(&mut World, Entity, Entity, u8);

pub fn on_hit(spell_id: u32) -> Option<HitHook> {
    match spell_id {
        FROSTBOLT => Some(frostbolt_chilled),
        _ => None,
    }
}

/// `spell_mage_frostbolt::HandleChilled` (spell_mage.cpp:978-995): the caster casts
/// Chilled at the hit unit, triggered.
fn frostbolt_chilled(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    super::cast_triggered(world, caster, CHILLED, Some(target), depth + 1);
}
