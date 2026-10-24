//! TC a352b1fa spell_shaman.cpp459–522/1871–1921: launch Maelstrom children.

use bevy::prelude::*;

use super::{TargetPlan, begin_validated_cast, load_triggered_spell, validate_start};
use crate::auras::ActiveAuras;
use crate::spell_info::SpellInfo;

const MAELSTROM_CONTROLLER: u32 = 343725;

/// Once per launch, not once per landed target. Chain uses selected effect0 units.
pub(super) fn energize_launch(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    plan: &TargetPlan,
    depth: u8,
) {
    let (controller_index, child, chain) = match spell.id {
        188196 => (0, 214815, false),
        45284 => (1, 214816, false),
        188443 => (4, 195897, true),
        45297 => (5, 218558, true),
        _ => return,
    };
    let Some(amount) = controller_amount(world, caster, controller_index) else {
        return;
    };
    let count = if chain {
        effect0_target_count(spell, plan)
    } else {
        1
    };
    cast_energize(world, caster, child, amount * count as f32, depth + 1);
}

fn controller_amount(world: &World, caster: Entity, index: u8) -> Option<f32> {
    let auras = world.get::<ActiveAuras>(caster)?;
    let controller = auras
        .iter()
        .find(|aura| aura.spell_id == MAELSTROM_CONTROLLER)?;
    controller
        .effects
        .iter()
        .find(|effect| effect.index == index)
        .map(|effect| effect.amount)
}

fn effect0_target_count(spell: &SpellInfo, plan: &TargetPlan) -> usize {
    plan.iter()
        .find(|planned| spell.effects[planned.index].index == 0)
        .map_or(0, |planned| planned.units.len())
}

fn cast_energize(world: &mut World, caster: Entity, child: u32, amount: f32, depth: u8) {
    let Some(mut spell) = load_triggered_spell(world, child, depth) else {
        return;
    };
    let Some(effect) = spell.effects.iter_mut().find(|effect| effect.index == 0) else {
        warn!(
            spell_id = child,
            "Maelstrom energize child missing native effect0"
        );
        return;
    };
    effect.base_points = amount;
    // Player.cpp22659–22661: nested helper modifiers belong to the triggering cast.
    spell.cast_parent_owns_modifier_charges = true;
    // Only IGNORE_CAST_IN_PROGRESS | DONT_REPORT_CAST_ERROR, not FULL_MASK.
    let result = validate_start(world, caster, &spell, Some(caster), true)
        .and_then(|target| begin_validated_cast(world, caster, &spell, target));
    if let Err(failure) = result {
        warn!(
            spell_id = child,
            ?failure,
            "Maelstrom energize child rejected"
        );
    }
}
