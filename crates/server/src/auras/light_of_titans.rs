//! Native378405/378412: Word of Glory's authored percentage becomes its HoT.
//! SimC76f8118 sc_paladin.cpp1129–1177/1251–1258 supplies the producer hook;
//! local70448 amounts and clock win over that reference's legacy values.
use bevy::prelude::*;

use super::AuraInstance;
use super::procs::AuraEvent;

pub(super) const LIGHT_OF_TITANS: u32 = 378_405;
const WORD_OF_GLORY: u32 = 85_673;

pub(super) fn check_word(_world: &World, _aura: &AuraInstance, event: &AuraEvent) -> bool {
    event.spell_id == Some(WORD_OF_GLORY) && event.heal.is_some_and(|heal| heal > 0.0)
}

pub(super) fn apply_word_hot(
    world: &mut World,
    _unit: Entity,
    aura: &AuraInstance,
    event: &AuraEvent,
) {
    let Some((heal, target)) = event.heal.zip(event.target) else {
        return;
    };
    let Some(effect) = aura.effects.iter().find(|effect| effect.index == 0) else {
        return;
    };
    let child = effect.trigger_spell;
    let Some(ticks) = super::periodic::heal_tick_count(world, event.actor, target, child) else {
        warn!("Light of the Titans {child} has no valid native periodic-heal clock");
        return;
    };
    let amount = heal * effect.amount / 100.0 / ticks as f32;
    crate::spell_cast::cast_triggered_with_effect0_amount(
        world,
        event.actor,
        child,
        Some(target),
        1,
        amount,
    );
}
