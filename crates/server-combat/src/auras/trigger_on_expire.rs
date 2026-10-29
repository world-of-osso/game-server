//! TC a352b1fa SpellAuraEffects.cpp AuraEffect::HandleTriggerSpellOnExpire (aura495).
use bevy::prelude::*;

use super::{AuraInstance, RemoveReason, handlers};

/// Only `AURA_REMOVE_BY_EXPIRE` casts. MiscValue0 selects the caster:
/// 0 the aura target, any other value the aura caster. The target is always
/// the aura target.
pub(super) fn on_removed(
    world: &mut World,
    target: Entity,
    aura: &AuraInstance,
    reason: RemoveReason,
) {
    if reason != RemoveReason::Expired {
        return;
    }
    for effect in &aura.effects {
        if effect.aura_type != handlers::TRIGGER_SPELL_ON_EXPIRE || effect.trigger_spell == 0 {
            continue;
        }
        let caster = if effect.misc_values[0] > 0 {
            aura.caster
        } else {
            target
        };
        if world.get_entity(caster).is_err() || world.get_entity(target).is_err() {
            continue;
        }
        crate::spell_cast::cast_triggered(world, caster, effect.trigger_spell, Some(target), 1);
    }
}
