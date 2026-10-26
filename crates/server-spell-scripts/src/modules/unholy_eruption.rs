//! Native70448 Scourge Strike eruption; SimC76f8118 sc_death_knight12502–12661.
//! Spread, Scourging, ghoul readiness and the simulator bug-mode divide are excluded.
use bevy::prelude::*;

use super::combat;
use combat::auras::ActiveAuras;
use combat::spell_cast::effects::{DamageInput, EffectCtx, damage_before_mitigation};
use combat::spell_cast::script_registry::{HitHook, SpellScriptHook};
use combat::spell_cast::scripts::script_effect_value;
use combat::spell_cast::{self, targets};
use combat::spell_info::SpellStore;

const SCOURGE_STRIKE: u32 = 55090;
const VIRULENT_PLAGUE: u32 = 191587;
pub const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::EffectHit(effect_hit)];

fn effect_hit(spell: u32, index: u8) -> Option<HitHook> {
    ((spell, index) == (SCOURGE_STRIKE, 0)).then_some(erupt_and_apply_missing)
}
const ERUPTIONS: &[(u32, u32)] = &[(191587, 1241167), (1240996, 1241171)];

pub(super) fn erupt_and_apply_missing(
    world: &mut World,
    caster: Entity,
    target: Entity,
    depth: u8,
) {
    if !targets::is_alive(world, target) {
        return;
    }
    let fraction = script_effect_value(world, caster, SCOURGE_STRIKE, 1) / 100.0;
    for &(plague, child) in ERUPTIONS {
        let Some(amount) = owned_tick_amount(world, caster, target, plague) else {
            continue;
        };
        spell_cast::cast_triggered_with_effect0_amount(
            world,
            caster,
            child,
            Some(target),
            depth + 1,
            amount * fraction,
        );
    }
    if targets::is_alive(world, target) {
        apply_missing_plague(world, caster, target, depth);
    }
}

///55090 applies Virulent Plague if absent. Existing marks belong to the separate
///spread/eruption contract; they are not refreshed as a substitute for spreading.
fn apply_missing_plague(world: &mut World, caster: Entity, target: Entity, _depth: u8) {
    let owned = world.get::<ActiveAuras>(target).is_some_and(|auras| {
        auras
            .iter()
            .any(|a| a.spell_id == VIRULENT_PLAGUE && a.caster == caster)
    });
    if !owned {
        combat::auras::apply_aura(
            world,
            &combat::auras::AuraApplication {
                caster,
                target,
                spell_id: VIRULENT_PLAGUE,
                effect_mask: 1,
                source: combat::auras::AuraSource::Cast,
            },
        );
    }
}

/// The stored aura amount has AP/base-point snapshots, but not periodic damage
/// modifiers. Resolve those once without executing a tick or consuming its clock.
/// Native children ignore caster percent-done, while retaining their own taken stage.
fn owned_tick_amount(world: &World, caster: Entity, target: Entity, plague: u32) -> Option<f32> {
    if !targets::is_alive(world, target) {
        return None;
    }
    let aura = world
        .get::<ActiveAuras>(target)?
        .iter()
        .find(|aura| aura.spell_id == plague && aura.caster == caster)?;
    let state = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0 && effect.period_ms > 0)?;
    let spell = world
        .resource::<SpellStore>()
        .get(plague)
        .expect("Scourge eruption requires the active plague's native spell");
    let effect = spell
        .effects
        .iter()
        .find(|effect| effect.index == state.index)
        .expect("active plague effect belongs to its native spell");
    let ctx = EffectCtx {
        caster,
        spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    let damage = DamageInput {
        amount: state.amount * aura.stack_multiplier(),
        crit: false,
        periodic: true,
        resolved_source: false,
        effect: Some(effect),
        bonus_caster: caster,
    };
    Some(damage_before_mitigation(world, &ctx, target, &damage))
}
