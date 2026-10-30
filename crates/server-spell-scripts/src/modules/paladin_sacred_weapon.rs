//! SimC76f8118 sc_paladin.cpp2401–2488, 2500–2520: Sacred Weapon procs strike
//! enemies or heal allies around the event's other party.
use super::combat;
use bevy::prelude::*;
use combat::auras::{AuraInstance, scripts::AuraScripts};
use combat::spell_cast::script_registry::{SpellScriptHook, TargetPlanHook};
use combat::spell_cast::targets::{self, Relation};
use combat::spell_cast::{PlannedEffect, TargetPlan};
use combat::spell_info::SpellInfo;
use std::collections::HashMap;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::DeclaredReach(declared_reach),
    SpellScriptHook::LateAuraRegistration(register),
    SpellScriptHook::TargetPlan(plan_hook),
];

const SACRED_WEAPON: u32 = 432502;
const STRIKE: u32 = 432616;
const HEAL: u32 = 441590;
// sc_paladin.hpp779: default solo Reflection of Radiance chance per strike.
const REFLECTION_CHANCE: f32 = 0.1;

fn declared_reach(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
) -> combat::spell_cast::script_registry::ScriptReach {
    if spell.id != STRIKE
        && !combat::spell_cast::script_registry::reach_source_selected(world, caster, SACRED_WEAPON)
    {
        return Default::default();
    }
    let strike = world
        .resource::<combat::spell_info::SpellStore>()
        .get(STRIKE)
        .expect("native Sacred Weapon strike");
    combat::spell_cast::script_registry::ScriptReach::target(targets::radius(&strike.effects[0], 1))
}

#[derive(Component, Default)]
struct SacredWeaponClocks(HashMap<Entity, combat::auras::ProcTimers>);

fn register(scripts: &mut AuraScripts) {
    let script = scripts.0.entry(SACRED_WEAPON).or_default();
    script.on_apply = Some(restore_weapon_clock);
    script.before_remove_reactions = Some(save_weapon_clock);
    script.check_proc = Some(accept_weapon_event);
    script.on_damage_proc = Some(wield_sacred_weapon);
    // SimC76f8118 sc_paladin.cpp2403–2408,2444–2449,2462–2467,2490–2496:
    // Sacred and Lesser Weapon child actions disable proc callbacks.
    for spell in [432616, 441590, 1239282, 1239276] {
        scripts.0.entry(spell).or_default().suppress_outgoing_procs = true;
    }
}

// SimC76f8118 sc_paladin.cpp2813–2822,3313–3314 keeps one callback
// across armament activations; its RPPM clock is not reset on expiry.
fn restore_weapon_clock(world: &mut World, unit: Entity, aura: &AuraInstance) {
    let timers = world
        .get::<SacredWeaponClocks>(unit)
        .and_then(|clocks| clocks.0.get(&aura.caster))
        .copied();
    if let Some(timers) = timers {
        combat::auras::procs::restore_proc_timers(world, unit, aura.instance_id, timers);
    }
}

fn save_weapon_clock(
    world: &mut World,
    unit: Entity,
    aura: &AuraInstance,
    _: combat::auras::RemoveReason,
) {
    let timers = combat::auras::procs::snapshot_proc_timers(aura);
    world
        .entity_mut(unit)
        .entry::<SacredWeaponClocks>()
        .or_default()
        .get_mut()
        .0
        .insert(aura.caster, timers);
}

fn accept_weapon_event(
    world: &World,
    _: &AuraInstance,
    event: &combat::auras::procs::AuraEvent,
) -> bool {
    if event.target != Some(event.actor) || event.damage > 0.0 || event.heal.is_some() {
        return true;
    }
    let Some(spell) = event
        .spell_id
        .and_then(|id| world.resource::<combat::spell_info::SpellStore>().get(id))
    else {
        return true;
    };
    // SimC76f8118 sc_paladin.cpp2509–2519 procs on the action's target.
    // A separate caster-only energize hit is not another helpful action.
    let self_effects = spell
        .effects
        .iter()
        .filter(|effect| effect.implicit_targets[0] == 1);
    let self_effects: Vec<_> = self_effects.collect();
    !(spell.effects.iter().any(|effect| effect.effect == 2)
        && !self_effects.is_empty()
        && self_effects.iter().all(|effect| effect.effect == 30))
}

fn wield_sacred_weapon(
    world: &mut World,
    unit: Entity,
    _aura: &AuraInstance,
    event: &combat::auras::procs::AuraEvent,
) {
    // SimC76f8118 sc_paladin_protection.cpp448–469: Blessed Hammer's
    // foreground action targets the enemy even though it energizes its caster.
    let target = if event.spell_id == Some(204019) {
        event.action_target
    } else {
        event.target
    };
    let Some(target) = target else {
        error!(?unit, spell = ?event.spell_id, "Sacred Weapon proc has no action target");
        return;
    };
    if targets::relation_matches(world, unit, target, Relation::Enemy) {
        combat::spell_cast::cast_triggered(world, unit, STRIKE, Some(target), 1);
        super::paladin_armaments::roll_reflection_of_radiance(world, unit, REFLECTION_CHANCE);
    } else {
        combat::spell_cast::cast_triggered(world, unit, HEAL, Some(target), 1);
    }
}

fn plan_hook(spell: u32) -> Option<(Relation, TargetPlanHook)> {
    match spell {
        STRIKE => Some((Relation::Enemy, plan_strike)),
        HEAL => Some((Relation::Ally, plan_heal)),
        _ => None,
    }
}

/// sc_paladin.cpp2407–2430: every enemy, soft-capped at effect1's five,
/// and 50% more against a lone enemy.
fn plan_strike(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    primary: Option<Entity>,
) -> TargetPlan {
    let cap = spell.effects[1].base_points;
    let units = units_around(world, caster, spell, primary, Relation::Enemy);
    let lone = if units.len() == 1 { 1.5 } else { 1.0 };
    let multiplier = combat::spell_cast::scripts::soft_cap_multiplier(units.len(), cap) * lone;
    plan(world, caster, spell, primary, units, multiplier)
}

/// sc_paladin.cpp2463–2478: up to effect1's five allies, doubled on a lone ally.
fn plan_heal(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    primary: Option<Entity>,
) -> TargetPlan {
    let mut units = units_around(world, caster, spell, primary, Relation::Ally);
    units.truncate(spell.effects[1].base_points as usize);
    let multiplier = if units.len() == 1 { 2.0 } else { 1.0 };
    plan(world, caster, spell, primary, units, multiplier)
}

fn units_around(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    primary: Option<Entity>,
    relation: Relation,
) -> Vec<Entity> {
    let primary = primary.expect("Sacred Weapon proc requires its event party");
    let center = targets::position(world, primary).expect("Sacred Weapon party position");
    // Native destination is slot0; its area selector/radius is slot1.
    let radius = targets::radius(&spell.effects[0], 1);
    let mut units = targets::area_units(world, caster, center, radius, relation, 0);
    // SimC AoE actions retain the selected primary first.
    units.retain(|unit| *unit != primary);
    units.insert(0, primary);
    units
}

fn plan(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    primary: Option<Entity>,
    units: Vec<Entity>,
    multiplier: f32,
) -> TargetPlan {
    vec![PlannedEffect {
        index: 0,
        chain_multipliers: units.iter().map(|&unit| (unit, multiplier)).collect(),
        units,
        selected_units: Vec::new(),
        dest: targets::effect_dest(
            world,
            caster,
            primary,
            &spell.effects[0],
            spell.cast_destination,
        ),
        blocked_units: Vec::new(),
        missed_units: Vec::new(),
        prevent_hit_script: false,
    }]
}
