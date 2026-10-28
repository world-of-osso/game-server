//! Consume the native one-cast Reversion replacement.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::script_registry::SpellScriptHook;

const BLESSING: u32 = 1256581;
const OVERRIDE: u32 = 1256579;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterNativeOnCast(consume_override)];

fn consume_override(world: &mut World, caster: Entity, spell: u32, depth: u8) {
    if spell != BLESSING || depth != 0 { return; }
    // Native talent1256577 description: only the next Reversion is upgraded.
    // Aura1256579 has no proc charges; its Attr9 override flag is client-only.
    combat::auras::remove_auras(world, caster, combat::auras::RemoveReason::Consumed,
        |aura| aura.spell_id == OVERRIDE && aura.caster == caster);
}
