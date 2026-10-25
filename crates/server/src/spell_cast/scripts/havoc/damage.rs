//! SimC76f8118 sc_demon_hunter.cpp2608–2620,3088–3110,6878,7040,7455,8258.
use std::collections::BTreeSet;

use bevy::prelude::*;

use crate::spell_info::SpellStore;

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
    if !crate::spell_cast::targets::relation_matches(
        world,
        caster,
        target,
        crate::spell_cast::targets::Relation::Enemy,
    ) {
        return;
    }
    apply_initiative(world, caster, target);
    if spell.is_none_or(|id| BURNING_HITS.contains(&id))
        && super::has_talent(world, caster, BURNING_BLADES)
    {
        apply_burning_blades(world, caster, target, amount);
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
        .0
        .insert(target);
    if first_hit {
        crate::spell_cast::cast_triggered(world, caster, INITIATIVE_BUFF, Some(caster), 1);
    }
}

fn apply_burning_blades(world: &mut World, caster: Entity, target: Entity, damage: f32) {
    let Some(ticks) = native_tick_count(world) else {
        error!("Native Burning Blades453177 has no positive tick count");
        return;
    };
    let percent = super::super::script_effect_value(world, caster, BURNING_BLADES, 0);
    // Use the landed amount in attacker units, not AuraEvent.damage's scaled
    // target-health units. Native453177's rolling-periodic flag carries unpaid
    // damage and preserves its existing tick phase through generic refresh.
    crate::spell_cast::cast_triggered_with_effect0_amount(
        world,
        caster,
        RESIDUAL,
        Some(target),
        1,
        damage * percent / 100.0 / ticks as f32,
    );
}

fn native_tick_count(world: &World) -> Option<u32> {
    let spell = world.resource::<SpellStore>().get(RESIDUAL)?;
    let duration = spell.misc.as_ref()?.duration?.duration_ms;
    let period = spell
        .effects
        .iter()
        .find(|effect| effect.index == 0)?
        .aura_period_ms;
    if duration <= 0 || period == 0 {
        return None;
    }
    let ticks = duration as u32 / period;
    (ticks > 0).then_some(ticks)
}
