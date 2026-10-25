//! Counts and named resource bridges from local Mainline69933 and70448.
use crate::spell_info::SpellInfo;
use bevy::prelude::*;

pub(super) const BARBED_SHOT: u32 = 217200;
const BARBED_FOCUS: u32 = 246152;
// Native70448 Pack321014 names instant Focus child1282660 in its description.
const PACK_TACTICS: u32 = 321014;
const PACK_FOCUS: u32 = 1282660;

///217200 explicitly names246152's amount, duration and period. Start the
///native periodic resource aura once per completed root cast, not per tick.
pub(super) fn grant_barbed_focus(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    crate::spell_cast::cast_triggered(world, caster, BARBED_FOCUS, Some(caster), 1);
    if crate::auras::has_aura(world, caster, PACK_TACTICS)
        && !crate::spell_cast::try_cast_triggered(world, caster, PACK_FOCUS, Some(caster), 1)
    {
        warn!(?caster, "Native Pack Tactics instant Focus child refused");
    }
}
