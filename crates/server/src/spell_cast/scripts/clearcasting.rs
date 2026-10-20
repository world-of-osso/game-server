//! Native113043: Lifebloom HoT rolls effect0's chance, not DB ProcChance100.
//! TC a352b1fa spell_druid.cpp1871–1882; SimC76f8118 sc_druid.cpp6247,11532–11535.
use bevy::prelude::*;

use crate::auras::{AuraInstance, RemoveReason, procs::AuraEvent, remove_auras};

pub(crate) const OMEN: u32 = 113043;
const CLEARCASTING: u32 = 16870;
const LIFEBLOOM: u32 = 33763;

pub(crate) fn check_omen_proc(_world: &World, aura: &AuraInstance, event: &AuraEvent) -> bool {
    let own_lifebloom_tick =
        event.periodic && event.spell_id == Some(LIFEBLOOM) && event.actor == aura.caster;
    if !own_lifebloom_tick {
        return false;
    }
    let chance = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Omen retains its chance effect")
        .amount;
    rand::random::<f32>() * 100.0 < chance
}

pub(super) fn regrowth_hit(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    super::soul_of_the_forest::consume_after_heal(world, caster, target, depth);
    remove_auras(world, caster, RemoveReason::Consumed, |aura| {
        aura.spell_id == CLEARCASTING && aura.caster == caster
    });
}
