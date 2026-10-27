//! SimC76f8118 sc_evoker.cpp3004–3006/3223–3250/6368–6393/10643–10648.
//! Native flight duration, four-use talent amount, flat cost and stack cap remain data-owned.
use super::combat;
use bevy::prelude::*;
use combat::auras::scripts::AuraScripts;
use combat::auras::{ActiveAuras, AuraInstance, RemoveReason, has_aura};
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::scripts::{AfterCastHook, script_effect_value};
use combat::spell_info::SpellInfo;
use shared::components::ActiveSpec;

const TALENT: u32 = 370781;
const DISCOUNT: u32 = 411055;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::ScriptedChannel(command_channel),
    SpellScriptHook::AfterResourceValues(breath_hook),
    SpellScriptHook::MidAuraRegistration(register),
    SpellScriptHook::CaptureConsumedDamageModifier(admit_paid_use),
    SpellScriptHook::PreserveUnadmittedProcs(&[DISCOUNT]),
];

fn command_flight(world: &World, caster: Entity) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 1467)
        && has_aura(world, caster, 433871)
        && has_aura(world, caster, 1260745)
}

fn command_channel(world: &World, caster: Entity, spell: &SpellInfo) -> bool {
    // sc_evoker.cpp6329–6334: selected Maneuverability + Command Squadron channels.
    spell.id == 433874 && command_flight(world, caster)
}

fn breath_hook(spell: u32) -> Option<AfterCastHook> {
    matches!(spell, 357210 | 433874).then_some(after_breath)
}

fn after_breath(world: &mut World, caster: Entity, spell: &SpellInfo) {
    // sc_evoker.cpp6389–6393: command flight grants on last_tick, not execute.
    if !command_channel(world, caster, spell) {
        grant_discount(world, caster);
    }
}

fn register(scripts: &mut AuraScripts) {
    scripts.0.entry(433874).or_default().on_remove = Some(finish_flight);
}

fn finish_flight(world: &mut World, unit: Entity, aura: &AuraInstance, reason: RemoveReason) {
    super::evoker_command_squadron::finish_maneuverability(world, unit, aura, reason);
    if unit == aura.caster
        && command_flight(world, unit)
        && matches!(reason, RemoveReason::ChannelEnded | RemoveReason::Expired)
    {
        // sc_evoker.cpp6368–6371: normal completion and actual channel cancellation.
        grant_discount(world, unit);
    }
}

fn grant_discount(world: &mut World, caster: Entity) {
    if !has_aura(world, caster, TALENT) {
        return;
    }
    let uses = script_effect_value(world, caster, TALENT, 0) as u8;
    assert!(
        uses > 0,
        "selected native Imminent Destruction initial uses"
    );
    let overrides = combat::spell_cast::TriggeredSpellOverrides {
        aura_stacks: Some(uses),
        ..Default::default()
    };
    if !combat::spell_cast::cast_triggered_with_overrides(
        world,
        caster,
        DISCOUNT,
        Some(caster),
        1,
        overrides,
    ) {
        warn!(?caster, "Native Imminent Destruction grant refused");
    }
}

fn admit_paid_use(
    world: &World,
    caster: Entity,
    spell: &mut SpellInfo,
    foreground: bool,
) -> Option<u32> {
    if !foreground || !matches!(spell.id, 356995 | 357211) || has_aura(world, caster, 359618) {
        return None;
    }
    let aura = world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.spell_id == DISCOUNT && aura.caster == caster)?;
    // sc_evoker.cpp3223–3250: Essence Burst wins; otherwise admit exactly this owned use.
    // Ordinary charged-mod consumption spends it, retaining other native stacks.
    spell.cast_proc_mods.push(aura.instance_id);
    None
}
