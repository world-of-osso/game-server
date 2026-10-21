//! SimC76f8118 sc_monk.cpp358–373,870–871,6359–6365; native70448 values.
use bevy::prelude::*;

use crate::auras::{self, ActiveAuras, RemoveReason};
use crate::random as rand;
use crate::spell_cast::SpellHistory;
use crate::spell_info::SpellInfo;

// Source may_combo_strike actions, shared with the published starter policy.
const COMBO_ACTIONS: &[u32] = &[100780, 100784, 107428, 113656, 101546, 392983, 322109];
const HIT_COMBO: u32 = 196740;
const HIT_COMBO_BUFF: u32 = 196741;
const COMBO_BREAKER: u32 = 137384;
const COMBO_BREAKER_BUFF: u32 = 116768;

/// Ordinary successful casts only; triggered children do not change combo order.
pub(crate) fn ordinary_combo_cast(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !COMBO_ACTIONS.contains(&spell.id) || !auras::has_aura(world, caster, HIT_COMBO) {
        return;
    }
    let previous = world
        .get::<SpellHistory>(caster)
        .and_then(|history| history.last_cast_in(COMBO_ACTIONS));
    if previous == Some(spell.id) {
        auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
            aura.spell_id == HIT_COMBO_BUFF && aura.caster == caster
        });
    } else {
        crate::spell_cast::cast_triggered(world, caster, HIT_COMBO_BUFF, Some(caster), 1);
    }
}

/// Tiger Palm rolls its learned passive's native effect0 chance on a landed hit.
pub(super) fn tiger_palm_hit(world: &mut World, caster: Entity, _target: Entity, depth: u8) {
    let chance = world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| auras.iter().find(|aura| aura.spell_id == COMBO_BREAKER))
        .and_then(|aura| aura.effects.iter().find(|effect| effect.index == 0))
        .map_or(0.0, |effect| effect.amount);
    if rand::random::<f32>() * 100.0 < chance {
        crate::spell_cast::cast_triggered(
            world,
            caster,
            COMBO_BREAKER_BUFF,
            Some(caster),
            depth + 1,
        );
    }
}
