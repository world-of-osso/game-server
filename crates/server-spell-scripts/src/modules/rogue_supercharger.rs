//! Native Rogue Supercharger counters and effective points, separate from payment.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, RemoveReason};
use combat::spell_cast::scripts::{AfterCastHook, script_effect_value};
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};
use combat::spell_info::SpellInfo;
use shared::components::PowerType;

const TALENT: u32 = 470347;
const FORCED_INDUCTION: u32 = 470668;
const ECHOING_REPRIMAND: u32 = 470669;
const ECHOING_BUFF: u32 = 470671;
// SimC76f8118 sc_rogue.cpp10961–10966: counters have a fixed source order.
const COUNTERS: &[u32] = &[470398, 470406, 470409, 470412, 470414, 470415, 470416];

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AuraRollAfterCast(after_cast),
    SpellScriptHook::AfterDirectDamageBeforeItems(after_kingsbane_hit),
    SpellScriptHook::EffectiveResourceValue(effective_points),
    SpellScriptHook::ConsumeResourceValueSources(consume_counter),
];

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    // sc_rogue.cpp3578/5576: ordinary Adrenaline Rush and Shadow Dance.
    matches!(spell, 13750 | 185313).then_some(grant_after_cast)
}

fn grant_after_cast(world: &mut World, caster: Entity, _: &SpellInfo) {
    grant_counters(world, caster);
}

fn after_kingsbane_hit(world: &mut World, ctx: &EffectCtx, _: Entity, _: bool) {
    // sc_rogue.cpp4947–4961: successful Kingsbane impact, never its DoT.
    if ctx.spell.id == 385627 && ctx.depth == 0 {
        grant_counters(world, ctx.caster);
    }
}

fn grant_counters(world: &mut World, caster: Entity) {
    if !auras::has_aura(world, caster, TALENT) {
        return;
    }
    let mut remaining = script_effect_value(world, caster, TALENT, 0) as usize;
    // sc_rogue.cpp8913–8933: fill the first inactive counters, not a stack
    // on an existing counter; an already-full set cannot earn additional uses.
    for &counter in COUNTERS {
        if remaining == 0 {
            break;
        }
        if auras::has_aura(world, caster, counter) {
            continue;
        }
        spell_cast::cast_triggered(world, caster, counter, Some(caster), 1);
        remaining -= 1;
    }
}

fn consumes_supercharger(spell: &SpellInfo) -> bool {
    let rogue = spell
        .class_options
        .as_ref()
        .is_some_and(|row| row.spell_class_set == 8);
    let spender = spell
        .power_costs
        .iter()
        .any(|cost| cost.power_type == PowerType::ComboPoints as i32);
    let damaging = spell
        .effects
        .iter()
        .any(|effect| effect.bonus_coefficient_from_ap > 0.0);
    // sc_rogue.cpp2215–2216 defaults to direct/tick AP finishers;
    //4851/5496/6912 explicitly include Spree, Secret Technique and Coup.
    rogue && spender && (damaging || matches!(spell.id, 51690 | 280719 | 441776))
}

fn read_last_counter(world: &World, caster: Entity) -> Option<u32> {
    let auras = world.get::<ActiveAuras>(caster)?;
    COUNTERS.iter().rev().find_map(|counter| {
        auras
            .iter()
            .find(|aura| aura.spell_id == *counter && aura.caster == caster)
            .map(|aura| aura.instance_id)
    })
}

fn effective_points(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    resource: PowerType,
    value: f32,
) -> f32 {
    if resource != PowerType::ComboPoints
        || value <= 0.0
        || !consumes_supercharger(spell)
        || !auras::has_aura(world, caster, TALENT)
        || read_last_counter(world, caster).is_none()
    {
        return value;
    }
    // sc_rogue.cpp2021–2038: add to the frozen effective value AFTER Fang
    // Strike's maximum; neither source changes the actual native CP payment.
    let extra = script_effect_value(world, caster, TALENT, 1);
    let forced = if auras::has_aura(world, caster, FORCED_INDUCTION) {
        script_effect_value(world, caster, FORCED_INDUCTION, 0)
    } else {
        0.0
    };
    value + extra + forced
}

fn consume_counter(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !consumes_supercharger(spell) || !auras::has_aura(world, caster, TALENT) {
        return;
    }
    let Some(instance) = read_last_counter(world, caster) else {
        return;
    };
    // sc_rogue.cpp8938–8961: spend only the highest active counter once
    // after a successful finisher and open the selected Echoing Reprimand buff.
    auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
        aura.instance_id == instance
    });
    if auras::has_aura(world, caster, ECHOING_REPRIMAND) {
        spell_cast::cast_triggered(world, caster, ECHOING_BUFF, Some(caster), 1);
    }
}
