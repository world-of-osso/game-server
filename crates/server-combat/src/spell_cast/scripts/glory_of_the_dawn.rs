//! SimC76f8118 sc_monk.cpp963–996: one haste-scaled roll after each kick execute.
use bevy::prelude::*;

use crate::class_progression::TraitLoadout;
use crate::spell_info::SpellInfo;
use crate::stats::CharacterStats;

const TALENT: u32 = 392958;
pub(super) const DAMAGE: u32 = 392959;

fn read_chance(world: &World, caster: Entity) -> Option<f32> {
    let selected = world
        .get::<TraitLoadout>(caster)?
        .active_talent_spells()
        .any(|spell| spell.spell_id == TALENT);
    if !selected {
        return None;
    }
    let coefficient = super::script_effect_value(world, caster, TALENT, 1);
    let haste = world
        .get::<CharacterStats>(caster)
        .expect("native Glory caster retains actual class haste")
        .haste_pct;
    Some(coefficient / 100.0 * haste / 100.0)
}

pub(super) fn after_launch(world: &mut World, caster: Entity, spell: &SpellInfo, depth: u8) {
    if !matches!(spell.id, 185099 | 468179) {
        return;
    }
    let Some(chance) = read_chance(world, caster) else {
        return;
    };
    if crate::random::random::<f32>() >= chance {
        return;
    }
    let Some(primary) = spell.cast_primary_target else {
        error!(
            ?caster,
            spell = spell.id,
            "Glory kick execute lacks its original primary"
        );
        return;
    };
    // The actual native child owns damage and effect2's Chi; no resource grant here.
    crate::spell_cast::cast_triggered(world, caster, DAMAGE, Some(primary), depth + 1);
}
