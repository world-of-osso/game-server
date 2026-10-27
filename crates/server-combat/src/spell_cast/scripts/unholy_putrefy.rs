//! SimC76f8118 sc_death_knight.cpp12229–12244: Putrefy summons a lesser ghoul.
use crate::spell_info::SpellInfo;
use bevy::prelude::*;

pub(super) fn after_cast_hook(spell: u32) -> Option<super::AfterCastHook> {
    (spell == 1247378).then_some(summon_lesser_ghoul)
}

fn summon_lesser_ghoul(world: &mut World, caster: Entity, spell: &SpellInfo) {
    cast_lesser_ghoul(world, caster);
    if !crate::auras::has_aura(world, caster, 377580) {
        return;
    }
    // SimC76f8118 sc_death_knight.cpp12229–12249: after the ordinary
    // payment, one remaining charge means Putrid Echoes spends it too.
    let remaining =
        crate::spell_cast::cooldown_query::cooldown_value(world, caster, spell, "charges")
            .expect("native Putrefy charge state");
    if remaining >= 1.0 {
        cast_lesser_ghoul(world, caster);
        crate::spell_cast::start_cooldowns(world, caster, spell);
    }
}

fn cast_lesser_ghoul(world: &mut World, caster: Entity) {
    if !crate::spell_cast::try_cast_triggered(world, caster, 1277098, Some(caster), 1) {
        warn!(?caster, "Native Putrefy lesser-ghoul summon refused");
    }
}
