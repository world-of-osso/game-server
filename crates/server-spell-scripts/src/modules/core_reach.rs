//! Reach declarations for the combat-owned script implementations.
use super::combat;
use combat::spell_cast::script_registry::SpellScriptHook;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::DeclaredReach(
    combat::spell_cast::scripts::native_script_reach,
)];
