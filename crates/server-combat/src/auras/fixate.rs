//! TC a352b1fa AuraEffect::HandleAuraModFixate: the aura caster's ThreatManager
//! fixates the aura target if it is on its threat list (`FixateTarget`), and any
//! removal clears the caster's fixate (`ClearFixate`).
use bevy::prelude::*;
use shared::components::ThreatTable;

use super::{AuraInstance, handlers};
use crate::creature_aggro::Fixate;

fn fixates(aura: &AuraInstance) -> bool {
    aura.effects
        .iter()
        .any(|effect| effect.aura_type == handlers::MOD_FIXATE)
}

pub(super) fn on_applied(world: &mut World, target: Entity, aura: &AuraInstance) {
    if !fixates(aura) {
        return;
    }
    let Ok(mut caster) = world.get_entity_mut(aura.caster) else {
        return;
    };
    let listed = caster.get::<ThreatTable>().is_some_and(|threat| {
        threat
            .entries()
            .iter()
            .any(|entry| entry.entity == target.to_bits())
    });
    if listed {
        caster.insert(Fixate(target));
    } else {
        caster.remove::<Fixate>();
    }
}

pub(super) fn on_removed(world: &mut World, aura: &AuraInstance) {
    if !fixates(aura) {
        return;
    }
    if let Ok(mut caster) = world.get_entity_mut(aura.caster) {
        caster.remove::<Fixate>();
    }
}
