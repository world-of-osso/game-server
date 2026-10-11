//! Counts and named resource bridges from local Mainline12.1.0.69933.
use crate::spell_info::SpellInfo;
use bevy::prelude::*;

pub(super) const BARBED_SHOT: u32 = 217200;
const BARBED_FOCUS: u32 = 246152;
const STORMKEEPER: u32 = 191634;

///217200 explicitly names246152's amount, duration and period. Start the
///native periodic resource aura once per completed root cast, not per tick.
pub(super) fn grant_barbed_focus(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    crate::spell_cast::cast_triggered(world, caster, BARBED_FOCUS, Some(caster), 1);
}

///191634s5 specifies two empowered spells; the options row's single charge
///does not implement that count. Ordinary matching spell mods consume uses.
pub(crate) fn proc_charges(world: &World, caster: Entity, spell: &SpellInfo, native: f32) -> f32 {
    if spell.id == STORMKEEPER {
        super::script_effect_value(world, caster, STORMKEEPER, 4)
    } else {
        native
    }
}
