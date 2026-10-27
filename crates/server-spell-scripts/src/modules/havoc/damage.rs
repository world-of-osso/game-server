//! SimC76f8118 sc_demon_hunter.cpp2608–2620,3088–3110,6878,7040,7455,8258.
use std::collections::BTreeSet;

use bevy::prelude::*;

use super::combat;

const BURNING_BLADES: u32 = 452408;
const RESIDUAL: u32 = 453177;
const INITIATIVE: u32 = 388108;
const INITIATIVE_BUFF: u32 = 391215;
// Chaos Strike/Annihilation children, First Blood's primary-only strikes,
// and Throw Glaive. Ordinary swings have no spell ID. Secondary Blade Dance
// damage does not inherit burning_blades_trigger_t in the pinned source.
const BURNING_HITS: &[u32] = &[
    222031, 199547, 227518, 201428, 391374, 391378, 393055, 393054, 337819,
];

#[derive(Component, Default)]
struct InitiativeTargets(BTreeSet<Entity>);

pub(super) fn before_cast(world: &mut World, caster: Entity, spell: u32) {
    // SimC76f8118 sc_demon_hunter.cpp8567–8576: reset before the real
    // Vengeful Retreat damage child executes, not after its hits.
    if spell == 198813 && super::has_talent(world, caster, INITIATIVE) {
        if let Some(mut targets) = world.get_mut::<InitiativeTargets>(caster) {
            targets.0.clear();
        }
    }
}

pub(crate) fn apply_havoc_damage_policies(
    world: &mut World,
    caster: Entity,
    target: Entity,
    spell: Option<u32>,
    amount: f32,
    periodic: bool,
) {
    if amount <= 0.0 || periodic || !super::is_havoc(world, caster) {
        return;
    }
    if !combat::spell_cast::targets::relation_matches(
        world,
        caster,
        target,
        combat::spell_cast::targets::Relation::Enemy,
    ) {
        return;
    }
    apply_initiative(world, caster, target);
    if spell.is_none_or(|id| BURNING_HITS.contains(&id))
        && super::has_talent(world, caster, BURNING_BLADES)
    {
        super::super::demon_hunter_burning::apply(world, caster, target, amount, RESIDUAL);
    }
}

fn apply_initiative(world: &mut World, caster: Entity, target: Entity) {
    if !super::has_talent(world, caster, INITIATIVE) {
        return;
    }
    let first_hit = world
        .entity_mut(caster)
        .entry::<InitiativeTargets>()
        .or_default()
        .into_mut()
        .0
        .insert(target);
    if first_hit {
        combat::spell_cast::cast_triggered(world, caster, INITIATIVE_BUFF, Some(caster), 1);
    }
}
