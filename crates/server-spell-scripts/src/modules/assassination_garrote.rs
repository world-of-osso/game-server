//! SimC76f8118 rogue4551–4561: Garrote's persistent factor is the native
//! Improved Garrote factor already frozen in its admitted cast and bleed amount.
use super::combat::spell_cast::script_registry::SpellScriptHook;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::PersistentCastMultiplier(&[703])];
