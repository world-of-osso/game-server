//! Pinned Windstrike exception to ordinary Physical armor classification.
use super::combat::spell_cast::script_registry::SpellScriptHook;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::ArmorIgnoredSpells(ARMOR_IGNORED)];

// SimC76f8118 sc_shaman.cpp4726/4743: Windstrike and Windlash ignore armor.
const ARMOR_IGNORED: &[u32] = &[115357, 115360, 114089, 114093];
