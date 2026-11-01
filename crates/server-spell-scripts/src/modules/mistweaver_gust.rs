//! Native70448 Mastery: Gust of Mists on the primary recipient of each mist.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{self, script_registry::SpellScriptHook, scripts::AfterCastHook};
use combat::spell_info::SpellInfo;

const MASTERY: u32 = 117_907;
const GUST: u32 = 191_894;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::NativeAfterCast(after_cast)];

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    matches!(spell, 115_151 | 124_682).then_some(trigger_gust)
}

fn trigger_gust(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !combat::auras::has_aura(world, caster, MASTERY) {
        return;
    }
    let Some(target) = spell.cast_primary_target else {
        warn!(?caster, spell = spell.id, "Gust of Mists lacks its primary target");
        return;
    };
    // 117907 effect 3 modifies 191894's spell-power coefficient by mastery;
    // the ordinary triggered heal applies that native modifier and healing bonuses.
    if !spell_cast::try_cast_triggered(world, caster, GUST, Some(target), 1) {
        warn!(?caster, ?target, "Gust of Mists trigger failed");
    }
}
