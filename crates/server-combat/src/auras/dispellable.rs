//! Native dispel capability from SpellEffect38 and active aura DispelType.
use super::{ActiveAuras, AuraInstance, aura_can_be_dispelled};
use crate::spell_cast::effects::EFFECT_DISPEL;
use crate::spell_info::SpellInfo;

/// Count removable debuffs once each, even when several spell effects match.
/// The caller supplies the simulation clock; health and debuff spell IDs do not select eligibility.
pub fn debuff_count(auras: &ActiveAuras, spell: &SpellInfo, now: f64) -> usize {
    auras
        .iter()
        .filter(|aura| aura.expires_at.is_none_or(|expiry| expiry > now))
        .filter(|aura| spell_removes_debuff(spell, aura))
        .count()
}

fn spell_removes_debuff(spell: &SpellInfo, aura: &AuraInstance) -> bool {
    spell
        .effects
        .iter()
        .filter(|effect| effect.effect == EFFECT_DISPEL)
        .filter_map(|effect| u32::try_from(effect.misc_values[0]).ok())
        .any(|dispel_type| aura_can_be_dispelled(aura, false, dispel_type))
}
