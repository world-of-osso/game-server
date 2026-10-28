//! SimC76f8118 sc_druid.cpp5355–5423: Lunar Beam heals once per ground pulse.
use bevy::prelude::*;

use super::combat;
use combat::spell_cast::{
    self, effects::EffectCtx, queued_triggers, script_registry::SpellScriptHook,
};
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] =
    &[SpellScriptHook::AfterForegroundSpecial(schedule_heals)];

fn schedule_heals(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if spell.id != 204066 {
        return;
    }
    let duration = spell_cast::duration_ms(world, caster, spell).max(0) as u32;
    let ctx = EffectCtx {
        caster,
        spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    for delay in (1000..=duration).step_by(1000) {
        queued_triggers::schedule_full_trigger(world, &ctx, caster, 204069, delay);
    }
}
