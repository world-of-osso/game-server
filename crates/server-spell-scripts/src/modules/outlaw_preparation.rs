//! Preparation resets the native Outlaw cooldowns.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::SpellStore;

const PREPARATION: u32 = 1277933;
const RESET_COOLDOWNS: &[u32] = &[13750, 315341, 13877, 271877, 51690];

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterNativeOnCast(reset_cooldowns)];

fn reset_cooldowns(world: &mut World, caster: Entity, spell: u32, depth: u8) {
    if spell != PREPARATION || depth != 0 {
        return;
    }
    // SimC76f8118 sc_rogue.cpp6419–6440: a real Preparation cast resets
    // these five running cooldowns; DB2 70448 effect1290409 is a dummy.
    for &id in RESET_COOLDOWNS {
        let now = spell_cast::now(world);
        let source = world
            .resource::<SpellStore>()
            .get(id)
            .expect("native Preparation cooldown");
        let remaining = world
            .get::<spell_cast::SpellHistory>(caster)
            .map_or(0, |history| history.remaining_cooldown_ms(source, now));
        spell_cast::scripts::reduce_cooldown(world, caster, id, remaining as f64 / 1000.0);
    }
}
