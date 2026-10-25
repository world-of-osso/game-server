//! TC a352b1fa spell_shaman.cpp2480–2510: Ghost Wolf + Spirit Wolf lifecycle.
use bevy::prelude::*;

use super::scripts::{AuraScript, AuraScripts};
use super::{AuraInstance, RemoveReason};

const GHOST_WOLF: u32 = 2_645;
const TALENT: u32 = 260_878;
const PERIODIC: u32 = 260_882;
const BUFF: u32 = 260_881;

pub(super) fn register(scripts: &mut AuraScripts) {
    for spell in [GHOST_WOLF, TALENT] {
        scripts.0.insert(
            spell,
            AuraScript {
                on_apply: Some(start_periodic_when_both_present),
                on_remove: Some(remove_children),
                ..Default::default()
            },
        );
    }
}

fn start_periodic_when_both_present(world: &mut World, unit: Entity, _aura: &AuraInstance) {
    if super::has_aura(world, unit, GHOST_WOLF) && super::has_aura(world, unit, TALENT) {
        crate::spell_cast::cast_triggered(world, unit, PERIODIC, Some(unit), 1);
    }
}

fn remove_children(world: &mut World, unit: Entity, _aura: &AuraInstance, _reason: RemoveReason) {
    super::remove_auras(world, unit, RemoveReason::Cancelled, |aura| {
        matches!(aura.spell_id, PERIODIC | BUFF)
    });
}
