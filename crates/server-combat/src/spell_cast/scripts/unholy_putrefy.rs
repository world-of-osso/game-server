//! SimC76f8118 sc_death_knight.cpp12229–12244: Putrefy summons a lesser ghoul.
use crate::spell_info::SpellInfo;
use bevy::prelude::*;

pub(super) fn after_cast_hook(spell: u32) -> Option<super::AfterCastHook> {
    (spell == 1247378).then_some(summon_lesser_ghoul)
}

fn summon_lesser_ghoul(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    if !crate::spell_cast::try_cast_triggered(world, caster, 1277098, Some(caster), 1) {
        warn!(?caster, "Native Putrefy lesser-ghoul summon refused");
    }
}
