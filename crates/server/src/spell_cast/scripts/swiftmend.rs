//! SimC76f8118 sc_druid.cpp6558–6571 own-HoT impact order.
//! Native392410 overrides consumption; consumed-HoT bonus healing is outside scope.
use bevy::prelude::*;

use crate::auras::{ActiveAuras, RemoveReason, has_aura, remove_instance};

pub(super) const SWIFTMEND: u32 = 18562;
const VERDANT_INFUSION: u32 = 392410;
const OWN_HOT_CONSUMPTION_ORDER: &[u32] = &[8936, 48438, 774];

pub(super) fn consume_own_hot(world: &mut World, caster: Entity, target: Entity, _depth: u8) {
    if has_aura(world, caster, VERDANT_INFUSION) {
        return;
    }
    let instance = world.get::<ActiveAuras>(target).and_then(|auras| {
        OWN_HOT_CONSUMPTION_ORDER.iter().find_map(|spell| {
            auras
                .iter()
                .find(|aura| aura.spell_id == *spell && aura.caster == caster)
                .map(|aura| aura.instance_id)
        })
    });
    if let Some(instance) = instance {
        remove_instance(world, target, instance, RemoveReason::Consumed);
    }
}
