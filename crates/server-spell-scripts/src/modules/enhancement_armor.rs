//! Pinned Enhancement damage classification, without changing other hybrid schools.
use super::combat::spell_cast::script_registry::SpellScriptHook;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::ArmorIgnoredSpells(ARMOR_IGNORED),
];

// SimC76f8118 sc_shaman.cpp4726: both Windstrike weapon children ignore armor.
// action.cpp1686–1689: only pure Physical actions use armor, not Doom's
// native469270 Physical/Nature hybrid. Preserve native coefficients and schools.
const ARMOR_IGNORED: &[u32] = &[115357, 115360, 469270];
