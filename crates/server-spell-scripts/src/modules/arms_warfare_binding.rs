//! Master of Warfare's script-bound native damage and critical-damage amounts.
use super::{combat, selected};
use bevy::prelude::*;
use combat::auras::AuraInstance;

pub(super) fn register(scripts: &mut combat::auras::scripts::AuraScripts) {
    scripts.0.entry(1269394).or_default().on_apply = Some(bind_modifiers);
}

fn bind_modifiers(world: &mut World, unit: Entity, aura: &AuraInstance) {
    if aura.caster != unit || !selected(world, unit, 1269306) {
        return;
    }
    // SimC76f8118 sc_warrior.cpp7888–7891/7902–7907: the selected
    // talent's native rank values are in tenths of a percent. Bind the
    // zero-valued buff effects; leave its separate armor penetration intact.
    for (source, target) in [(0, 1), (1, 2)] {
        let amount =
            combat::spell_cast::scripts::script_effect_value(world, unit, 1269306, source) / 10.0;
        combat::auras::set_effect_amount(world, unit, aura.instance_id, target, amount);
    }
}
