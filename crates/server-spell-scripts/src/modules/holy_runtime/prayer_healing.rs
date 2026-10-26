//! Native Prayer of Healing injured-ally plan.
use super::{PRAYER_OF_HEALING, combat, script_effect_value};
use bevy::prelude::*;
use combat::spell_cast::{
    PlannedEffect, TargetPlan,
    targets::{self, Relation},
};
use combat::spell_info::SpellInfo;

fn select_prayer_allies(
    world: &mut World,
    caster: Entity,
    primary: Entity,
    radius: f32,
    additional: usize,
) -> Vec<Entity> {
    let center = targets::position(world, primary).expect("validated ally position");
    let mut units = targets::area_units(world, caster, center, radius, Relation::Ally, 0);
    units.retain(|&unit| {
        unit != primary
            && world
                .get::<shared::components::Health>(unit)
                .is_some_and(|health| health.current < health.max)
    });
    units.truncate(additional);
    units.insert(0, primary);
    units
}

/// 596: target plus s3-1 injured allies inside effect1's second radius.
/// Data does not specify an injury ordering; retain normal nearest-first
/// area selection, exclude full-health units and cap the additional allies.
pub(super) fn prayer_plan(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    explicit: Option<Entity>,
) -> TargetPlan {
    let primary = explicit.expect("Prayer of Healing validated ally");
    let heal = spell
        .effects
        .iter()
        .position(|effect| effect.index == 1)
        .expect("Prayer heal effect");
    let radius = targets::radius(&spell.effects[heal], 1);
    let additional = script_effect_value(world, caster, PRAYER_OF_HEALING, 2) as usize - 1;
    let units = select_prayer_allies(world, caster, primary, radius, additional);
    vec![
        PlannedEffect {
            index: 0,
            units: vec![primary],
            dest: None,
            chain_multipliers: Vec::new(),
            selected_units: Vec::new(),
            blocked_units: Vec::new(),
            prevent_hit_script: false,
        },
        PlannedEffect {
            index: heal,
            units,
            dest: None,
            chain_multipliers: Vec::new(),
            selected_units: Vec::new(),
            blocked_units: Vec::new(),
            prevent_hit_script: false,
        },
    ]
}
