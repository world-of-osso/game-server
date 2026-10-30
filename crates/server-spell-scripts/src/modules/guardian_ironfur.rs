//! SimC76f8118 sc_druid.cpp11012–11017 overrides native Ironfur 192081 with
//! asynchronous stacks: each cast expires on its own 7-second lifetime instead of
//! refreshing every stack.
use super::combat;
use combat::spell_cast::script_registry::SpellScriptHook;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AsynchronousAuras(&[192081])];
