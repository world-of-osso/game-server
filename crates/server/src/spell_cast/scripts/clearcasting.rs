//! Native113043: Lifebloom HoT rolls effect0's chance, not DB ProcChance100.
//! TC a352b1fa spell_druid.cpp1871–1882; SimC76f8118 sc_druid.cpp6247,11532–11535.
use crate::random as rand;
use bevy::prelude::*;

use crate::auras::{ActiveAuras, AuraInstance, procs::AuraEvent, spend_stack};

pub(crate) const OMEN: u32 = 113043;
const CLEARCASTING: u32 = 16870;
const LIFEBLOOM: u32 = 33763;
const TRANQUIL_MIND: u32 = 403521;

pub(crate) fn check_omen_proc(world: &World, aura: &AuraInstance, event: &AuraEvent) -> bool {
    let own_lifebloom_tick =
        event.periodic && event.spell_id == Some(LIFEBLOOM) && event.actor == aura.caster;
    if !own_lifebloom_tick {
        return false;
    }
    let chance = tranquil_mind_chance(world, aura.caster).unwrap_or_else(|| {
        aura.effects
            .iter()
            .find(|effect| effect.index == 0)
            .expect("native Omen retains its chance effect")
            .amount
    });
    rand::random::<f32>() * 100.0 < chance
}

/// Native403521s3 replaces Omen's chance; s1's existing op37 supplies the cap.
fn tranquil_mind_chance(world: &World, caster: Entity) -> Option<f32> {
    world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.spell_id == TRANQUIL_MIND)
        .map(|talent| {
            talent
                .effects
                .iter()
                .find(|effect| effect.index == 2)
                .expect("native Tranquil Mind retains its chance effect")
                .amount
        })
}

pub(super) fn regrowth_hit(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    super::soul_of_the_forest::consume_after_heal(world, caster, target, depth);
    let instance = world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == CLEARCASTING && aura.caster == caster)
        })
        .map(|aura| aura.instance_id);
    if let Some(instance) = instance {
        spend_stack(world, caster, instance);
    }
}
