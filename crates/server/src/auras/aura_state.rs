//! Computable target states from TC a352b1fa SpellInfo::_LoadAuraState.
//! TARGET_AURA_STATE_UNCOMPUTED: all states except15 are deliberately unknown.
use bevy::prelude::*;

use super::ActiveAuras;
use crate::spell_info::SpellStore;

pub(crate) const DRUID_PERIODIC_HEAL: u32 = 15;
const SPELL_FAMILY_DRUID: u32 = 7;
const FAERIE_FIRE_CATEGORY: u32 = 1133;

/// State15 is not in TC's PER_CASTER_AURA_STATE_MASK (only14/16 are).
/// Any caster's active matching aura qualifies; unknown states are not false.
pub(crate) fn read_computed_state(world: &World, target: Entity, state: u32) -> Option<bool> {
    if state != DRUID_PERIODIC_HEAL {
        return None;
    }
    let present = world.get::<ActiveAuras>(target).is_some_and(|auras| {
        auras
            .iter()
            .any(|aura| grants_druid_periodic_heal(world, aura.spell_id))
    });
    Some(present)
}

fn grants_druid_periodic_heal(world: &World, spell_id: u32) -> bool {
    let Some(spell) = world
        .get_resource::<SpellStore>()
        .and_then(|store| store.get(spell_id))
    else {
        return false;
    };
    // TC's category1133 classification precedes the Druid-family test.
    if spell
        .categories
        .as_ref()
        .is_some_and(|category| category.category == FAERIE_FIRE_CATEGORY)
    {
        return false;
    }
    spell.class_options.as_ref().is_some_and(|options| {
        let regrowth_or_rejuvenation = options.spell_class_mask[0] & 0x50 != 0;
        let wild_growth = options.spell_class_mask[1] & 0x0400_0000 != 0;
        options.spell_class_set == SPELL_FAMILY_DRUID && (regrowth_or_rejuvenation || wild_growth)
    })
}
