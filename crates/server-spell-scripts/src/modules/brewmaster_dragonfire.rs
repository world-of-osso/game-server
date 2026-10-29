//! SimC76f8118 sc_monk.cpp3257–3303: Breath of Fire schedules Dragonfire Brew.
use super::combat;
use bevy::prelude::*;
use combat::class_progression::TraitLoadout;
use combat::spell_cast::{self, script_registry::SpellScriptHook, scripts::AfterCastHook};
use combat::spell_info::SpellInfo;
use shared::components::ActiveSpec;

const TALENT: u32 = 383994;
const DAMAGE: u32 = 387621;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterResourceValues(cast_hook)];

fn cast_hook(spell: u32) -> Option<AfterCastHook> {
    (spell == 115181).then_some(schedule_dragonfire)
}

fn schedule_dragonfire(world: &mut World, caster: Entity, parent: &SpellInfo) {
    if !world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 268)
        || !world.get::<TraitLoadout>(caster).is_some_and(|traits| {
            traits
                .active_talent_spells()
                .any(|talent| talent.spell_id == TALENT)
        })
    {
        return;
    }
    let Some(target) = parent.cast_primary_target else {
        return;
    };
    let Some(position) = spell_cast::targets::position(world, target) else {
        return;
    };
    let count = combat::spell_cast::scripts::script_effect_value(world, caster, TALENT, 0) as usize;
    for index in 1..=count {
        let Some(mut spell) = spell_cast::load_triggered_spell(world, DAMAGE, 1) else {
            return;
        };
        spell
            .misc
            .as_mut()
            .expect("native Dragonfire Brew misc")
            .launch_delay = 1.5 * index as f32;
        spell.cast_destination = Some(position.to_array());
        spell_cast::launch_triggered_spell(world, caster, &spell, Some(target), 1);
    }
}
