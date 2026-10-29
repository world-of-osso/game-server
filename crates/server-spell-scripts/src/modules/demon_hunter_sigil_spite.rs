//! Native Sigil of Spite detonation on the actual ground trigger's expiry.
use super::combat;
use bevy::prelude::*;
use combat::soul_fragments::{self, FragmentKind};
use combat::spell_cast::{self, script_registry::SpellScriptHook};

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AreaTriggerExpired(detonate)];

fn detonate(world: &mut World, caster: Entity, spell: u32, position: Vec3) {
    if spell != 390163 {
        return;
    }
    // SimC76f8118 sc_demon_hunter.cpp5419–5445,11406: the sigil detonates
    // its native damage spell and creates effect3 lesser fragments once.
    let count = spell_cast::scripts::script_effect_value(world, caster, spell, 2);
    if !count.is_finite() || count < 0.0 || count.fract() != 0.0 {
        error!(spell, count, "invalid native Sigil of Spite fragment count");
        return;
    }
    spell_cast::cast_triggered_at_destination(world, caster, 389860, position, 1);
    if let Err(reason) =
        soul_fragments::spawn_many(world, caster, FragmentKind::Lesser, count as u32, false)
    {
        error!(spell, %reason, "Sigil of Spite fragment spawn failed");
    }
}
