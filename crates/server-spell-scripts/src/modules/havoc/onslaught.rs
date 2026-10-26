//! SimC76f8118 sc_demon_hunter.cpp7422–7446,7462–7473,7524–7532.
use super::{
    combat,
    havoc::{has_talent, owned_passive_instance},
};
use bevy::prelude::*;
use combat::spell_cast::{effects::EffectCtx, queued_triggers, script_registry::SpellScriptHook};
use combat::spell_info::SpellStore;

const TALENT: u32 = 389977;
pub(super) const HOOKS: &[SpellScriptHook] =
    &[SpellScriptHook::OwnsPolicyProc(|spell| spell == TALENT)];

pub(super) fn roll_from_offhand(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    target: Entity,
    amount: f32,
) {
    let parent = match ctx.spell.id {
        199547 => 162794,
        201428 => 201427,
        _ => return,
    };
    if amount <= 0.0 || !has_talent(world, ctx.caster, TALENT) {
        return;
    }
    let Some(instance) = owned_passive_instance(world, ctx.caster, TALENT) else {
        return;
    };
    let chance = combat::spell_cast::scripts::script_effect_value(world, ctx.caster, TALENT, 0);
    // Roll at the landed off-hand hit, but do not spend the native ICD yet.
    if !combat::auras::procs::policy_proc_ready(world, ctx.caster, instance)
        || combat::random::random::<f32>() * 100.0 >= chance
    {
        return;
    }
    // The source uses this same off-hand effect's native delay a second time.
    let delay = read_offhand_delay(world, parent, ctx.spell.id);
    queued_triggers::schedule_callback(world, ctx, target, delay, execute_copy);
}

fn read_offhand_delay(world: &World, parent: u32, child: u32) -> u32 {
    let delay = world
        .resource::<SpellStore>()
        .get(parent)
        .expect("native Chaos Strike/Annihilation parent")
        .effects
        .iter()
        .find(|effect| effect.trigger_spell == child)
        .expect("native off-hand trigger")
        .misc_values[0];
    assert!(
        delay >= 0,
        "native Onslaught off-hand delay cannot be negative"
    );
    delay as u32
}

fn execute_copy(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    let Some(instance) = owned_passive_instance(world, caster, TALENT) else {
        return;
    };
    // SimC7433–7446 selects the live form at delayed execution, not the hit.
    let spell = if combat::auras::has_aura(world, caster, 162264) {
        201427
    } else {
        162794
    };
    if !combat::spell_cast::try_cast_triggered(world, caster, spell, Some(target), depth) {
        warn!(spell, "Native Relentless Onslaught copy refused");
    }
    // Its native1s ICD starts here. The copy's300ms OH cannot self-proc.
    if combat::auras::procs::start_policy_proc(world, caster, instance).is_none() {
        error!(
            ?caster,
            "Native Onslaught ICD could not start at its delayed execution"
        );
    }
}
