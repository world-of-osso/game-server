//! SimC76f8118 sc_mage.cpp recipient contracts; native rows own hit values/ranges.
use crate::auras;
use crate::spell_cast::{PlannedEffect, TargetPlan, powers, targets};
use crate::spell_info::SpellInfo;
use bevy::prelude::*;
use shared::components::PowerType;

fn plan_effect(index: usize, units: Vec<Entity>, secondary_multiplier: f32) -> PlannedEffect {
    let chain_multipliers = units
        .iter()
        .enumerate()
        .map(|(index, &unit)| {
            (
                unit,
                if index == 0 {
                    1.0
                } else {
                    secondary_multiplier
                },
            )
        })
        .collect();
    PlannedEffect {
        index,
        units,
        dest: None,
        chain_multipliers,
        selected_units: Vec::new(),
        blocked_units: Vec::new(),
        prevent_hit_script: false,
    }
}

pub(super) fn ranged_recipients(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    primary: Entity,
) -> Vec<Entity> {
    let center = targets::position(world, caster).expect("native Arcane caster position");
    let (_, maximum) = targets::spell_range(spell, true);
    let mut units =
        targets::area_units(world, caster, center, maximum, targets::Relation::Enemy, 0);
    units.retain(|&unit| unit != primary);
    units.insert(0, primary);
    units
}

pub(super) fn barrage(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    primary: Option<Entity>,
) -> TargetPlan {
    let primary = primary.expect("native Barrage primary");
    let mut units = ranged_recipients(world, caster, spell, primary);
    // sc_mage.cpp2711–2714: snapshot target count BEFORE Orb Barrage changes charges.
    let charges = powers::power_value(world, caster, PowerType::ArcaneCharges)
        .expect("Arcane Barrage requires its native pool")
        .0;
    let cleave = super::arcane::talent_value(world, caster, 231564, 1);
    let count = if cleave > 0.0 {
        1 + charges as usize
    } else {
        1
    };
    units.truncate(count);
    let damage = spell
        .effects
        .iter()
        .position(|effect| effect.index == 0)
        .unwrap();
    vec![plan_effect(damage, units, cleave / 100.0)]
}

pub(super) fn missiles(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    primary: Option<Entity>,
) -> TargetPlan {
    let primary = primary.expect("native Missiles tick primary");
    let damage = spell
        .effects
        .iter()
        .position(|effect| effect.index == 0)
        .unwrap();
    let mut plan = vec![plan_effect(damage, vec![primary], 1.0)];
    let extra = super::arcane::talent_value(world, caster, 453600, 1) as usize;
    let overpowered = if auras::has_aura(world, caster, 1277009) {
        super::script_effect_value(world, caster, 1277009, 1) as usize
    } else {
        0
    };
    // sc_mage.cpp3114–3147: use one full primary effect and native half-damage
    // secondary effect, not both effects against the same primary.
    let Some(index) = spell.effects.iter().position(|effect| effect.index == 1) else {
        return plan;
    };
    let effect = &spell.effects[index];
    let mut units = targets::select_targets(world, caster, Some(primary), spell, effect, 0, false)
        .expect("native Missiles secondary selection");
    units.retain(|&unit| unit != primary);
    units.truncate(extra + overpowered);
    plan.push(plan_effect(index, units, 1.0));
    plan
}

pub(super) fn prismatic(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    primary: Option<Entity>,
) -> TargetPlan {
    let primary = primary.expect("native Prismatic splash primary");
    let index = spell
        .effects
        .iter()
        .position(|effect| effect.index == 1)
        .unwrap();
    let effect = &spell.effects[index];
    let mut units = targets::select_targets(world, caster, Some(primary), spell, effect, 0, false)
        .expect("native Prismatic secondary selection");
    // sc_mage.cpp2905–2918: the separate native splash hits secondary targets only.
    units.retain(|&unit| unit != primary);
    vec![plan_effect(index, units, 1.0)]
}
