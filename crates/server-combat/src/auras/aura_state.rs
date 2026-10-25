//! Computable target states from TC a352b1fa SpellInfo::_LoadAuraState.
//! TC Unit.cpp481–487 computes seven health states; all other states except15
//! remain TARGET_AURA_STATE_UNCOMPUTED (unknown, not false).
use bevy::prelude::*;
use shared::components::Health;

#[cfg(test)]
mod tests;

use super::ActiveAuras;
use crate::spell_info::SpellStore;

pub(crate) const DRUID_PERIODIC_HEAL: u32 = 15;
const WOUNDED_20_PERCENT: u32 = 2;
const WOUNDED_25_PERCENT: u32 = 6;
const WOUNDED_35_PERCENT: u32 = 13;
const WOUND_HEALTH_20_80: u32 = 21;
const HEALTHY_75_PERCENT: u32 = 23;
const WOUND_HEALTH_35_80: u32 = 24;
const WOUNDED_50_PERCENT: u32 = 25;
const SPELL_FAMILY_DRUID: u32 = 7;
const FAERIE_FIRE_CATEGORY: u32 = 1133;

/// State15 is not in TC's PER_CASTER_AURA_STATE_MASK (only14/16 are).
/// Any caster's active matching aura qualifies; unknown states are not false.
pub(crate) fn read_computed_state(world: &World, target: Entity, state: u32) -> Option<bool> {
    if state != DRUID_PERIODIC_HEAL {
        return read_health_state(world.get::<Health>(target)?, state);
    }
    let present = world.get::<ActiveAuras>(target).is_some_and(|auras| {
        auras
            .iter()
            .any(|aura| grants_druid_periodic_heal(world, aura.spell_id))
    });
    Some(present)
}

/// TC Unit.h804–809: strict comparisons against uint64 CountPctFromMaxHealth.
/// Read current backing health every time; never persist stale threshold flags.
fn read_health_state(health: &Health, state: u32) -> Option<bool> {
    let current = f64::from(health.current).trunc();
    let maximum = f64::from(health.max).trunc();
    let count = |percent: f64| (maximum * percent / 100.0).trunc();
    Some(match state {
        WOUNDED_20_PERCENT => current < count(20.0),
        WOUNDED_25_PERCENT => current < count(25.0),
        WOUNDED_35_PERCENT => current < count(35.0),
        WOUND_HEALTH_20_80 => current < count(20.0) || current > count(80.0),
        HEALTHY_75_PERCENT => current > count(75.0),
        WOUND_HEALTH_35_80 => current < count(35.0) || current > count(80.0),
        WOUNDED_50_PERCENT => current < count(50.0),
        _ => return None,
    })
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
