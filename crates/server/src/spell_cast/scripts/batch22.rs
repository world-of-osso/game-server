//! Mainline363534s4: Rewind healing is increased outside a raid.
use bevy::prelude::*;

use crate::group::GroupRegistry;
use crate::persistence::ClientId;
use crate::spell_info::SpellInfo;

const REWIND: u32 = 363_534;
const NON_RAID_HEALING_PERCENT: u8 = 3;

/// Applies to the existing native heal, not a substitute for Rewind's missing
/// incoming-damage history. Group state is the caster's, not the recipient's.
pub(in crate::spell_cast) fn healing_multiplier(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
) -> f32 {
    if spell.id != REWIND {
        return 1.0;
    }
    let in_raid = world
        .get::<ClientId>(caster)
        .is_some_and(|id| world.resource::<GroupRegistry>().is_in_raid(id.0));
    if in_raid {
        return 1.0;
    }
    let effect = spell
        .effects
        .iter()
        .find(|effect| effect.index == NON_RAID_HEALING_PERCENT)
        .expect("Rewind requires its authored non-raid healing percentage");
    let percent = crate::spell_cast::effects::modded_base_points(world, caster, spell, effect);
    1.0 + percent / 100.0
}
