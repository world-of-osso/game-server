//! Native Havoc hit policies and per-ability runtime state.
use crate::class_progression::KnownSpells;
use bevy::prelude::*;
use shared::components::ActiveSpec;

mod damage;
mod dance;
mod demonsurge;
pub(crate) use damage::apply_havoc_damage_policies;
pub(crate) use dance::{filter_dance_targets, first_blood_damage, trigger_dance};
pub use demonsurge::havoc_demonsurge_available;
pub(crate) use demonsurge::register_havoc;

fn is_havoc(world: &World, caster: Entity) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 577)
}

fn has_talent(world: &World, caster: Entity, talent: u32) -> bool {
    is_havoc(world, caster)
        && world
            .get::<KnownSpells>(caster)
            .is_some_and(|known| known.contains(talent))
}

pub(crate) fn is_eye_beam_channel(spell: u32) -> bool {
    matches!(spell, super::EYE_BEAM | 452497)
}

pub fn before_cast(world: &mut World, caster: Entity, spell: u32, depth: u8) {
    damage::before_cast(world, caster, spell);
    demonsurge::before_cast(world, caster, spell, depth);
}

pub fn arm_demonic(world: &mut World, caster: Entity) {
    demonsurge::arm_demonic(world, caster);
}
