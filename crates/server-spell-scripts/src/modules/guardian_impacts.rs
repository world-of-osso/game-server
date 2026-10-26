//! Source-qualified guardian impact relationships absent from native Dummy rows.
use super::combat;
use combat::spell_cast::script_registry::{HitHook, SpellScriptHook, TargetPlanHook};
use combat::spell_cast::{self, PlannedEffect, TargetPlan, targets};
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::EffectHit(hit_hook),
    SpellScriptHook::TargetPlan(plan_hook),
];
use bevy::prelude::*;

struct Impact {
    parent: u32,
    effect: u8,
    child: u32,
}
// SimC76f8118 pets.cpp2006–2025: Soul Barrage casts native1292391 at
// up to the parent's actual effect0 base-value target count. No copied damage.
const IMPACTS: &[Impact] = &[Impact {
    parent: 1292384,
    effect: 0,
    child: 1292391,
}];

pub(super) fn hit_hook(spell: u32, effect: u8) -> Option<HitHook> {
    IMPACTS
        .iter()
        .any(|row| row.parent == spell && row.effect == effect)
        .then_some(launch_child)
}
fn plan_hook(spell: u32) -> Option<(targets::Relation, TargetPlanHook)> {
    IMPACTS
        .iter()
        .any(|row| row.parent == spell)
        .then_some((targets::Relation::Enemy, select_targets))
}

fn select_targets(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    primary: Option<Entity>,
) -> TargetPlan {
    let policy = IMPACTS.iter().find(|row| row.parent == spell.id).unwrap();
    let index = spell
        .effects
        .iter()
        .position(|effect| effect.index == policy.effect)
        .unwrap();
    let cap = spell.effects[index].base_points as usize;
    assert!(
        cap > 0,
        "native guardian impact requires a positive authored target count"
    );
    let units = select_capped_units(world, caster, spell, primary, cap);
    vec![PlannedEffect {
        index,
        units,
        selected_units: Vec::new(),
        dest: targets::effect_dest(
            world,
            caster,
            primary,
            &spell.effects[index],
            spell.cast_destination,
        ),
        chain_multipliers: Vec::new(),
        blocked_units: Vec::new(),
        prevent_hit_script: false,
    }]
}

fn select_capped_units(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    primary: Option<Entity>,
    cap: usize,
) -> Vec<Entity> {
    let center = targets::position(world, caster).expect("guardian requires native Position");
    let (_, range) = targets::spell_range(spell, true);
    let mut units = targets::area_units(world, caster, center, range, targets::Relation::Enemy, 0);
    // SimC AoE actions retain the selected primary, then admit secondary enemies.
    if let Some(primary) = primary.filter(|unit| units.contains(unit)) {
        units.retain(|unit| *unit != primary);
        units.insert(0, primary);
    }
    units.truncate(cap);
    units
}

fn launch_child(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    let parent = spell_cast::resolving_spell_id(world, caster)
        .expect("guardian impact requires its native resolving frame");
    let child = IMPACTS
        .iter()
        .find(|row| row.parent == parent)
        .unwrap()
        .child;
    if !spell_cast::try_cast_triggered(world, caster, child, Some(target), depth + 1) {
        warn!(
            ?caster,
            ?target,
            parent,
            child,
            "Native guardian impact child refused"
        );
    }
}
