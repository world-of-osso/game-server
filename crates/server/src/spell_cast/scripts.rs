//! SpellScript `OnHit` hooks keyed by spell ID (TrinityCore `spell_script_names`):
//! behaviour a spell has only through its script, run once per unit it lands on.

use bevy::prelude::*;

const FROSTBOLT: u32 = 116;
/// Chilled: MOD_DECREASE_SPEED 33 by 50% for 8 s.
const MAGE_CHILLED: u32 = 205708;

pub(super) fn on_hit(world: &mut World, caster: Entity, spell_id: u32, target: Entity, depth: u8) {
    // spell_mage_frostbolt::HandleChilled (spell_mage.cpp:986-990, TrinityCore a352b1fa).
    if spell_id == FROSTBOLT {
        super::cast_triggered(world, caster, MAGE_CHILLED, Some(target), depth + 1);
    }
}
