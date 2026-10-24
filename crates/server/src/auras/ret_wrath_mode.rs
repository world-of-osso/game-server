//! SIMC_RET_WRATH_HAMMER_MODE_PRODUCER.
//! SimC76f8118 sc_paladin.cpp314–315/3634–3641; native1241288 is Dummy/Trigger0.
//! Native aura durations and spell-override metadata remain untouched.
use bevy::prelude::*;

use super::scripts::{AuraScript, AuraScripts};
use super::{AuraInstance, RemoveReason};
use crate::class_progression::KnownSpells;

const HAMMER_TALENT: u32 = 1241288;
const HAMMER_MODE: u32 = 1277026;
const WRATHS: &[u32] = &[31884, 454351];

pub(super) fn register(scripts: &mut AuraScripts) {
    for &parent in WRATHS {
        scripts.0.insert(
            parent,
            AuraScript {
                on_apply: Some(apply_mode),
                on_remove: Some(remove_mode),
                ..Default::default()
            },
        );
    }
}

fn apply_mode(world: &mut World, unit: Entity, _parent: &AuraInstance) {
    let learned = world
        .get::<KnownSpells>(unit)
        .is_some_and(|known| known.contains(HAMMER_TALENT));
    if learned {
        let depth = crate::spell_cast::next_trigger_depth(world);
        crate::spell_cast::cast_triggered(world, unit, HAMMER_MODE, Some(unit), depth);
    }
}

fn remove_mode(world: &mut World, unit: Entity, _parent: &AuraInstance, reason: RemoveReason) {
    // Same caster-owned cleanup boundary as native linked-aura children; no
    // copied Wrath timer or new mode duration, and no global aura removal.
    super::remove_auras(world, unit, reason, |child| {
        child.spell_id == HAMMER_MODE && child.caster == unit
    });
}
