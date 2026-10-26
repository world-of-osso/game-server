//! Ordinary spell GCD haste precedes the separately authored aura417 path.
use super::*;

const SPELL_GCD_CATEGORY: u32 = 133;
const MELEE_DEFENSE: u32 = 2;
const RANGED_DEFENSE: u32 = 3;
const USES_RANGED_SLOT: u32 = 0x2;
const IS_ABILITY: u32 = 0x10;

pub(super) fn ordinary_spell_ms(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    gcd_ms: u32,
) -> u32 {
    // TC a352b1fa Spell.cpp9209–9218; SimC76f8118 action.cpp648–666,
    //1285–1315: ordinary spells haste by default, without requiring aura417.
    let defense = spell.categories.as_ref().map_or(0, |row| row.defense_type);
    let weapon_ability = matches!(defense, MELEE_DEFENSE | RANGED_DEFENSE)
        || attributes(spell, 0) & (USES_RANGED_SLOT | IS_ABILITY) != 0;
    if gcd_ms <= MIN_GCD_MS || gcd_category(spell) != SPELL_GCD_CATEGORY || weapon_ability {
        return gcd_ms;
    }
    let haste = spell_haste_multiplier(world, caster);
    ((gcd_ms as f32 / haste.max(0.01)) as u32).clamp(MIN_GCD_MS, MAX_GCD_MS)
}
