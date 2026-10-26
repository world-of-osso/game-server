//! SimC76f8118 sc_shaman.cpp6687–6694/7462–7469/9978–9985: refund actual paid MW.
use super::combat;
use bevy::prelude::*;
use shared::components::ActiveSpec;

use combat::auras;
use combat::spell_info::SpellInfo;

const TALENT: u32 = 1262635;

pub(super) fn refund(world: &mut World, caster: Entity, spell: &SpellInfo, spent: u8) {
    // Only normal/TI LB, CL and Tempest reach this spending seam; Arc/RTL never spend MW.
    if spent == 0 || !matches!(spell.id, 188196 | 188443 | 452201) {
        return;
    }
    let enhancement = world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 263);
    let selected = auras::has_aura(world, caster, TALENT);
    if !enhancement || !selected {
        return;
    }
    let chance = super::script_effect_value(world, caster, TALENT, 1);
    if combat::random::random::<f32>() * 100.0 < chance {
        super::maelstrom_weapon::generate(world, caster, spent);
    }
}
