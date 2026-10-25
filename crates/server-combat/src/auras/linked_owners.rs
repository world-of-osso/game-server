//! Shared child auras remain owned until the last matching native parent is gone.
use bevy::prelude::*;

use super::{ActiveAuras, linked};

pub(super) fn has_other_parent(
    world: &World,
    target: Entity,
    child_caster: Entity,
    child_spell: u32,
) -> bool {
    // SimC76f8118 sc_shaman.cpp12840–12860: overlapping modes retain their shared child.
    // Inspect native link metadata rather than special-casing either mode or its child spell.
    world.get::<ActiveAuras>(target).is_some_and(|active| {
        active.iter().any(|parent| {
            parent.effects.iter().any(|effect| {
                linked::is_linked(effect.aura_type)
                    && effect.trigger_spell == child_spell
                    && linked::find_child_caster(world, target, parent, child_spell)
                        == Some(child_caster)
            })
        })
    })
}
