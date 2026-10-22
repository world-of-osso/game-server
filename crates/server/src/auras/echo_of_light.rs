//! HOLY_ECHO_RESIDUAL_ON_NATIVE_AMOUNT: SimC76f8118 sc_priest.hpp1464–1476
//! supplies the producer shape; local77485 amount and77489 clock win.
use bevy::prelude::*;

use super::AuraInstance;
use super::procs::AuraEvent;

pub(super) const ECHO_MASTERY: u32 = 77_485;
pub(super) const ECHO_HEAL: u32 = 77_489;

pub(super) fn check_heal(_world: &World, _aura: &AuraInstance, event: &AuraEvent) -> bool {
    event.heal.is_some_and(|heal| heal > 0.0)
}

pub(super) fn apply_echo(world: &mut World, _unit: Entity, aura: &AuraInstance, event: &AuraEvent) {
    let Some((heal, target)) = event.heal.zip(event.target) else {
        return;
    };
    let Some(effect) = aura.effects.iter().find(|effect| effect.index == 0) else {
        return;
    };
    let total = heal * effect.amount / 100.0;
    if total <= 0.0 {
        return;
    }
    let Some(ticks) = super::periodic::heal_tick_count(world, event.actor, target, ECHO_HEAL)
    else {
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
