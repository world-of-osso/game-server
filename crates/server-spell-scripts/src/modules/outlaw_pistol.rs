//! Earned Pistol Shot uses and delayed Fan the Hammer background shots.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, spell_mods};
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};
use combat::spell_cast::queued_triggers::{self, ScaledTrigger};
use combat::spell_cast::scripts::{AfterCastHook, script_effect_value};
use combat::spell_info::SpellInfo;

const PISTOL_SHOT: u32 = 185763;
const OPPORTUNITY: u32 = 195627;
const FAN_THE_HAMMER: u32 = 381846;
const QUICK_DRAW: u32 = 196938;
const QUICK_DRAW_POINTS: u32 = 1254567;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterNativeOnCast(grant_quick_draw),
    SpellScriptHook::AuraRollAfterCast(after_cast),
    SpellScriptHook::CaptureConsumedDamageModifier(snapshot_background_modifiers),
    SpellScriptHook::FinishConsumedDamageModifier(consume_background_use),
];

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    (spell == PISTOL_SHOT).then_some(schedule_fan)
}

fn read_opportunity(world: &World, caster: Entity) -> Option<&auras::AuraInstance> {
    world.get::<ActiveAuras>(caster)?.iter()
        .find(|aura| aura.spell_id == OPPORTUNITY && aura.caster == caster)
}

fn grant_quick_draw(world: &mut World, caster: Entity, spell: u32, depth: u8) {
    if spell != PISTOL_SHOT || !auras::has_aura(world, caster, QUICK_DRAW)
        || read_opportunity(world, caster).is_none() {
        return;
    }
    // SimC76f8118 sc_rogue.cpp5042–5048: each earned shot pays the
    // native Quick Draw energize; this includes Fan's background shots.
    spell_cast::cast_triggered(world, caster, QUICK_DRAW_POINTS, Some(caster), depth + 1);
}

fn schedule_fan(world: &mut World, caster: Entity, spell: &SpellInfo) {
    let Some(opportunity) = read_opportunity(world, caster) else { return; };
    let remaining = opportunity.stacks.saturating_sub(1);
    if remaining == 0 || !auras::has_aura(world, caster, FAN_THE_HAMMER) { return; }
    let maximum = script_effect_value(world, caster, FAN_THE_HAMMER, 0) as u8;
    let multiplier = 1.0 - script_effect_value(world, caster, FAN_THE_HAMMER, 2) / 100.0;
    let target = spell.cast_primary_target.expect("native Pistol Shot primary target");
    let ctx = EffectCtx { caster, spell, depth: 0, dest: None, chain_multiplier: 1.0 };
    // sc_rogue.cpp5057–5069: ordinary execution spends one stack, then
    // schedules up to the native Fan count100ms apart. Children cannot fan.
    for shot in 0..remaining.min(maximum) {
        queued_triggers::schedule_scaled_full_trigger(world, &ctx, ScaledTrigger {
            target, spell_id: PISTOL_SHOT, delay_ms: 100 * (u32::from(shot) + 1), multiplier,
        });
    }
}

fn snapshot_background_modifiers(world: &World, caster: Entity, spell: &mut SpellInfo, foreground: bool) -> Option<u32> {
    if spell.id == PISTOL_SHOT && !foreground {
        // A triggered shot must freeze the same native charged modifiers as
        // its foreground root before consuming its own Opportunity use.
        spell.cast_charge_mods = Some(spell_mods::snapshot_charged_modifiers(world, caster, spell));
    }
    None
}

fn consume_background_use(world: &mut World, caster: Entity, spell: &SpellInfo, _: Option<u32>, _: u8, foreground: bool) {
    if spell.id != PISTOL_SHOT || foreground { return; }
    let instance = read_opportunity(world, caster).map(|aura| aura.instance_id);
    if let Some(instance) = instance {
        // sc_rogue.cpp5057: consume on execution, including a missed shot;
        // the frozen modifier remains available through projectile flight.
        auras::spend_stack(world, caster, instance);
    }
}
