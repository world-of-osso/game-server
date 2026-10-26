//! MID2 Winding Up is earned by MS/Overpower and fully consumed by Slam-family.
use super::{combat, selected};
use bevy::prelude::*;
use combat::auras::{ActiveAuras, RemoveReason, procs};
use combat::spell_info::SpellInfo;

const FOUR_PIECE: u32 = 1296644;
const WINDING_UP: u32 = 1300670;

pub(super) fn owns_proc(spell: u32) -> bool {
    spell == FOUR_PIECE
}

pub(super) fn after_cast(world: &mut World, caster: Entity, spell: &SpellInfo, _: bool) {
    if !selected(world, caster, FOUR_PIECE) {
        return;
    }
    match spell.id {
        // SimC76f8118 sc_warrior.cpp3111–3112/5494–5495: one
        // deterministic grant per MS/Overpower execution, including background MS.
        12294 | 7384 => grant_stack(world, caster),
        // sc_warrior.cpp3537–3538: expire the complete buff after Slam,
        // including inherited Heroic; a Slam hit must not produce a new stack.
        1464 | 1269383 => {
            combat::auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
                aura.spell_id == WINDING_UP && aura.caster == caster
            });
        }
        _ => {}
    }
}

fn grant_stack(world: &mut World, caster: Entity) {
    let instance = world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == FOUR_PIECE && aura.caster == caster)
                .map(|aura| aura.instance_id)
        })
        .expect("equipped native Arms4pc aura");
    // Retain the native source's500ms ICD; native1300670 owns magnitude/cap.
    if procs::try_policy_proc(world, caster, instance, 100.0)
        && !combat::spell_cast::try_cast_triggered(world, caster, WINDING_UP, Some(caster), 1)
    {
        warn!(?caster, "Native Winding Up grant refused");
    }
}
