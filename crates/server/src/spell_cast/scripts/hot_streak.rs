//! TC a352b1fa spell_mage.cpp1037–1129, with native70448 driver195283 (Effect3).
//! SimC76f8118 sc_mage.cpp2427: only the cast admitted with Hot Streak spends it.
use bevy::prelude::*;

use crate::auras::{ActiveAuras, RemoveReason, remove_instance};
use crate::class_progression::KnownSpells;
use crate::spell_info::SpellInfo;

const DRIVER: u32 = 195283;
const HEATING_UP: u32 = 48107;
pub(crate) const HOT_STREAK: u32 = 48108;
pub(super) const PYROBLAST: u32 = 11366;
pub(super) const FLAMESTRIKE: u32 = 1254851;

pub(crate) fn record_fire_hit(world: &mut World, caster: Entity, spell: &SpellInfo, crit: bool) {
    // SimC marks only Fireball, Fire Blast, Pyroblast and Scorch as HS triggers.
    let eligible = matches!(spell.id, 133 | 108853 | 319836 | PYROBLAST | 2948);
    let learned = world
        .get::<KnownSpells>(caster)
        .is_some_and(|known| known.contains(DRIVER));
    if !eligible || !learned {
        return;
    }
    let heating_up = world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == HEATING_UP && aura.caster == caster)
        })
        .map(|aura| aura.instance_id);
    if let Some(instance) = heating_up {
        remove_instance(world, caster, instance, RemoveReason::Consumed);
    }
    if crit {
        let proc = if heating_up.is_some() {
            HOT_STREAK
        } else {
            HEATING_UP
        };
        crate::spell_cast::cast_triggered(world, caster, proc, Some(caster), 1);
    }
}

pub(crate) fn consume_admitted_hot_streak(world: &mut World, caster: Entity, spell: &SpellInfo) {
    let instance = world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| {
            auras.iter().find(|aura| {
                aura.spell_id == HOT_STREAK && spell.cast_proc_mods.contains(&aura.instance_id)
            })
        })
        .map(|aura| aura.instance_id);
    if let Some(instance) = instance {
        remove_instance(world, caster, instance, RemoveReason::Consumed);
    }
}
