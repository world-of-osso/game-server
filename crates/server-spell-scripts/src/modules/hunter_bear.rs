//! Pack Leader Bear's native summon-time Rend Flesh.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::script_registry::{SpellScriptHook, TargetPlanHook};
use combat::spell_cast::{self, PlannedEffect, TargetPlan, summon::SummonedCreature, targets};
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterElementalLaunch(rend_on_summon),
    SpellScriptHook::TargetPlan(rend_plan),
];

const BEAR_SUMMON: u32 = 471990;
const REND_FLESH: u32 = 471999;

#[derive(Component)]
struct RendStarted;

fn rend_on_summon(world: &mut World, owner: Entity, spell: &SpellInfo, _: bool) {
    if spell.id != BEAR_SUMMON {
        return;
    }
    let Some(primary) = spell.cast_primary_target else {
        return;
    };
    let bears: Vec<_> = world
        .query_filtered::<(Entity, &SummonedCreature), Without<RendStarted>>()
        .iter(world)
        .filter(|(_, summon)| summon.owner == owner && summon.spell_id == BEAR_SUMMON)
        .map(|(bear, _)| bear)
        .collect();
    for bear in bears {
        // SimC76f8118 sc_hunter.cpp2057–2060: the actual new Bear, not
        // its owner or stable pet, immediately executes Rend on its target.
        if spell_cast::try_cast_triggered(world, bear, REND_FLESH, Some(primary), 1) {
            world.entity_mut(bear).insert(RendStarted);
        } else {
            warn!(?owner, ?bear, ?primary, "Native Bear Rend Flesh refused");
        }
    }
}

fn rend_plan(spell: u32) -> Option<(targets::Relation, TargetPlanHook)> {
    (spell == REND_FLESH).then_some((targets::Relation::Enemy, select_rend_targets))
}

fn select_rend_targets(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    primary: Option<Entity>,
) -> TargetPlan {
    let Some(primary) = primary else {
        return Vec::new();
    };
    let Some(center) = targets::position(world, primary) else {
        return Vec::new();
    };
    // SimC76f8118 sc_hunter.cpp3224: native effect2 supplies the cap;
    // the native periodic effect supplies its radius and damage coefficient.
    let radius = targets::effect_radius(&spell.effects[0]);
    let cap = spell.effects[1].base_points as usize;
    let mut units = targets::area_units(world, caster, center, radius, targets::Relation::Enemy, 0);
    if let Some(index) = units.iter().position(|unit| *unit == primary) {
        units.swap(0, index);
    }
    units.truncate(cap);
    vec![PlannedEffect {
        index: 0,
        units,
        chain_multipliers: Vec::new(),
        dest: None,
        selected_units: Vec::new(),
        blocked_units: Vec::new(),
        prevent_hit_script: false,
    }]
}
