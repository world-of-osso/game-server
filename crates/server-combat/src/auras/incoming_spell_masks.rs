//! Native incoming spell-damage effects honor both the source family and effect class mask.
use super::{AuraEffectState, AuraInstance, spell_mods};
use crate::spell_info::SpellInfo;

// SimC76f8118 sc_warrior.cpp1723–1734 applies Colossus Smash in the common
// Warrior target multiplier, not only to its native effect's Mortal Strike mask.
const UNIVERSAL_OWNER_DAMAGE: &[u32] = &[208086];

pub(super) fn affects_incoming_spell(
    aura: &AuraInstance,
    effect: &AuraEffectState,
    spell: &SpellInfo,
) -> bool {
    // SimC76f8118 sc_shaman.cpp7010–7028 scopes Burning Core to Fire Nova;
    // sc_hunter.cpp1278/1493–1494 parses Sentinel's separate native damage masks.
    UNIVERSAL_OWNER_DAMAGE.contains(&aura.spell_id)
        || spell_mods::class_mask_affects(aura, effect, spell)
}
