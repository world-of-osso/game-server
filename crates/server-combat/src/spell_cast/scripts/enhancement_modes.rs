//! SimC76f8118 sc_shaman.cpp8782–8790/12848–12869:
//! the actual Doom Winds aura emits its native area pulse on apply and native ticks.
use bevy::prelude::*;
use shared::components::Health;

use crate::auras::AuraInstance;
use crate::auras::scripts::{AuraScript, AuraScripts};
use crate::networking::CombatTarget;
use crate::spell_cast;

const DOOM_WINDS: u32 = 466772;
const DOOM_DAMAGE: u32 = 469270;

pub(super) fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        DOOM_WINDS,
        AuraScript {
            on_apply: Some(pulse),
            on_periodic: Some(tick),
            ..Default::default()
        },
    );
}

fn tick(world: &mut World, unit: Entity, aura: &AuraInstance, index: u8) {
    if index == 2 {
        pulse(world, unit, aura);
    }
}

fn pulse(world: &mut World, _unit: Entity, aura: &AuraInstance) {
    // SimC12864: a sleeping selected target suppresses this tick, never substitutes another unit.
    let selected = world
        .get::<CombatTarget>(aura.caster)
        .and_then(|target| target.0);
    let Some(target) = selected.filter(|target| {
        world
            .get::<Health>(*target)
            .is_some_and(|health| health.current > 0.0)
    }) else {
        return;
    };
    // Native469270 supplies caster-front destination, area selection and damage.
    spell_cast::cast_triggered(world, aura.caster, DOOM_DAMAGE, Some(target), 1);
}
