//! TC a352b1fa SpellAuraEffects.cpp3663: only players and pets have base armor.
use crate::hunter_pet::HunterPet;
use crate::spell_cast::summon::SummonedCreature;
use crate::stats::StatModifiers;

/// Non-player armor consumers apply this only to native pet ownership categories.
pub(crate) fn pet_multiplier(
    modifiers: Option<&StatModifiers>,
    hunter: Option<&HunterPet>,
    summon: Option<&SummonedCreature>,
) -> f32 {
    let is_pet = hunter.is_some() || summon.is_some_and(|unit| unit.properties.is_pet());
    if !is_pet {
        return 1.0;
    }
    modifiers.map_or(1.0, |mods| 1.0 + mods.base_armor_pct / 100.0)
}
