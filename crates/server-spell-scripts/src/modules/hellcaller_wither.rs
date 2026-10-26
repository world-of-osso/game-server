//! Shared Hellcaller Wither counter semantics, not a damage multiplier.
use super::combat::spell_cast::script_registry::SpellScriptHook;

// SimC76f8118 sc_warlock_actions.cpp1066 ignores counter stacks for tick
// damage;1174–1186 cancels refresh's automatic+1 without discarding earned
// stacks. The original native coefficient, phase and lifetime remain unchanged.
pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::CounterOnlyDotStacks(&[445474])];
