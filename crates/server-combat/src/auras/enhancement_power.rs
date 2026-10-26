//! SimC76f8118 sc_shaman.cpp11766–11770/12655–12658: one native haste stack
//! per actual MW-spending execution; later stacks do not refresh the current buff epoch.
use bevy::prelude::*;

use super::{ActiveAuras, AuraInstance, aura_now};
use crate::class_progression::KnownSpells;

const UNLIMITED_POWER: u32 = 454391;
const HASTE: u32 = 454394;

pub fn grant_for_spending(world: &mut World, caster: Entity, spent: u8) {
    if spent == 0
        || !world
            .get::<KnownSpells>(caster)
            .is_some_and(|known| known.contains(UNLIMITED_POWER))
    {
        return;
    }
    // Native454394 supplies the amount, stack cap, duration and haste handler.
    crate::spell_cast::cast_triggered(world, caster, HASTE, Some(caster), 1);
}

pub(super) fn preserve_refresh_duration(world: &World, target: Entity, new: &mut AuraInstance) {
    if new.spell_id != HASTE {
        return;
    }
    let now = aura_now(world);
    let previous = world.get::<ActiveAuras>(target).and_then(|auras| {
        auras.iter().find(|old| {
            old.spell_id == HASTE
                && old.caster == new.caster
                && old.source == new.source
                && old.expires_at.is_some_and(|expiry| expiry > now)
        })
    });
    if let Some(previous) = previous {
        new.expires_at = previous.expires_at;
    }
}
