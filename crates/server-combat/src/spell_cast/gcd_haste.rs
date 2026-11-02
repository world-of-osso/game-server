//! Ordinary spell GCD haste precedes the separately authored aura417 path.
use super::*;

const SPELL_GCD_CATEGORY: u32 = 133;
const MELEE_DEFENSE: u32 = 2;
const RANGED_DEFENSE: u32 = 3;
const USES_RANGED_SLOT: u32 = 0x2;
const IS_ABILITY: u32 = 0x10;

/// SimC 76f8118 sc_rogue.cpp2292–2306: Adrenaline Rush reduces the
/// affected action's GCD by up to 200 ms, proportional to haste up to 25%.
/// WCL Dm6KVw74QjfaAF3R fight 2 actor 8 shows sub-second casts during it.
pub(super) fn adrenaline_rush_ms(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    gcd_ms: u32,
) -> u32 {
    if gcd_ms == 0 {
        return 0;
    }
    let affects = world
        .get::<crate::auras::ActiveAuras>(caster)
        .is_some_and(|auras| {
            auras.iter().any(|aura| {
                aura.spell_id == 13750
                    && aura.caster == caster
                    && aura.effects.iter().any(|effect| {
                        effect.index == 2 && spell_mods::class_mask_affects(aura, effect, spell)
                    })
            })
        });
    if !affects {
        return gcd_ms;
    }
    let haste = (spell_haste_multiplier(world, caster) - 1.0).clamp(0.0, 0.25);
    gcd_ms.saturating_sub((200.0 * haste / 0.25).round() as u32)
}

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
