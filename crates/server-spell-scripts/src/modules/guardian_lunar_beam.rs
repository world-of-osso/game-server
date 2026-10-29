//! SimC76f8118 sc_druid.cpp5355–5423: Lunar Beam heals once per ground pulse.
use bevy::prelude::*;

use super::combat;
use combat::spell_cast::{
    self, effects::EffectCtx, queued_triggers, script_registry::SpellScriptHook, targets,
};
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterForegroundSpecial(schedule_heals),
    SpellScriptHook::TargetFilter(limit_pulse_targets),
];

// SimC76f8118 sc_druid.cpp5381–5385: pulse cap is Lunar Beam effect 3.
fn limit_pulse_targets(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    effect: u8,
    _: Option<Entity>,
    targets: &mut Vec<Entity>,
) {
    if spell.id == 414613 && effect == 0 {
        let limit =
            spell_cast::scripts::script_effect_value(world, caster, 204066, 2).max(0.0) as usize;
        targets.truncate(limit);
    }
}

fn schedule_heals(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if spell.id != 204066 {
        return;
    }
    let duration = spell_cast::duration_ms(world, caster, spell).max(0) as u32;
    let Some(origin) = targets::position(world, caster) else {
        return;
    };
    let ctx = EffectCtx {
        caster,
        spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    for delay in (1000..=duration).step_by(1000) {
        queued_triggers::schedule_destination_trigger(
            world,
            &ctx,
            queued_triggers::DestinationTrigger {
                target: caster,
                spell_id: 414613,
                delay_ms: delay,
                destination: origin.to_array(),
            },
        );
        queued_triggers::schedule_full_trigger(world, &ctx, caster, 204069, delay);
    }
}
