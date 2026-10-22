//! HOLY_ECHO_RESIDUAL_ON_NATIVE_AMOUNT: SimC76f8118 sc_priest.hpp1464–1476
//! supplies the producer shape; local77485 amount and77489 clock win.
use bevy::prelude::*;

use super::procs::AuraEvent;
use super::{ActiveAuras, AuraApplication, AuraInstance, AuraSource, aura_now, handlers};
use crate::spell_info::SpellStore;

pub(super) const ECHO_MASTERY: u32 = 77_485;
pub(super) const ECHO_HEAL: u32 = 77_489;
const PRISMATIC_ECHOES: u32 = 390_967;

/// SimC76f8118 sc_priest.cpp3704–3711: multiply Echo, not raw mastery/Renew.
fn prismatic_multiplier(world: &World, caster: Entity) -> f32 {
    let percent = world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == PRISMATIC_ECHOES && aura.caster == caster)
        })
        .and_then(|aura| aura.effects.iter().find(|effect| effect.index == 0))
        .map_or(0.0, |effect| effect.amount);
    1.0 + percent / 100.0
}

pub(super) fn check_heal(_world: &World, _aura: &AuraInstance, event: &AuraEvent) -> bool {
    event.heal.is_some_and(|heal| heal > 0.0)
}

fn read_tick_count(world: &World, caster: Entity, target: Entity) -> Option<u32> {
    let spell = world.get_resource::<SpellStore>()?.get(ECHO_HEAL)?;
    let effect = spell
        .effects
        .iter()
        .find(|effect| effect.index == 0 && effect.aura == handlers::PERIODIC_HEAL)?;
    let application = AuraApplication {
        caster,
        target,
        spell_id: ECHO_HEAL,
        effect_mask: 1,
        source: AuraSource::Cast,
    };
    let duration = super::duration_ms(world, &application, spell);
    let periodic = super::build_effect(world, caster, spell, effect, aura_now(world));
    duration
        .checked_div(periodic.period_ms)
        .filter(|ticks| *ticks > 0)
}

pub(super) fn apply_echo(world: &mut World, _unit: Entity, aura: &AuraInstance, event: &AuraEvent) {
    let Some((heal, target)) = event.heal.zip(event.target) else {
        return;
    };
    let Some(effect) = aura.effects.iter().find(|effect| effect.index == 0) else {
        return;
    };
    let total = heal * effect.amount / 100.0 * prismatic_multiplier(world, event.actor);
    if total <= 0.0 {
        return;
    }
    let Some(ticks) = read_tick_count(world, event.actor, target) else {
        warn!("Echo of Light {ECHO_HEAL} has no valid native periodic-heal clock");
        return;
    };
    // The real child carries the amount and owns its native aura/tick lifetime.
    // Unlike Binding Heals, Echo includes direct self healing.
    crate::spell_cast::cast_triggered_with_effect0_amount(
        world,
        event.actor,
        ECHO_HEAL,
        Some(target),
        1,
        total / ticks as f32,
    );
}
