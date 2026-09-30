//! Spell travel time: TrinityCore `Spell::HandleDelayed`. A spell with a missile
//! speed launches (SpellGo, CastSuccess, the caster's combat, costs) at once and lands on each
//! unit target after its hit delay (`Spell::AddUnitTarget`, Spell.cpp:2486-2518):
//! `LaunchDelay + max(max(distance, 5) / Speed, MinDuration)` seconds, or
//! `LaunchDelay + max(Speed, MinDuration)` with `SPELL_ATTR9_MISSILE_SPEED_IS_DELAY_IN_SEC`.
//! A spell aimed at a destination is one missile that hits all its targets together
//! after the flight to the destination (`CalculateDelayMomentForDst`, Spell.cpp:880-896;
//! `handle_delayed` `single_missile`, Spell.cpp:4095-4096). The caster's own target
//! entry has no delay ("Incoming time is zero for self casts").
//!
//! The delay is fixed at launch, as in TC: a target that moves is still hit when its
//! delay runs out, wherever it is. At the hit a target that left the world is skipped
//! (`TargetInfo::PreprocessTarget`, Spell.cpp:2746-2748), as is one whose alive state
//! changed in flight (`DoTargetSpellHit`, Spell.cpp:2807); a caster that left the world
//! drops the spell (`UpdatePointers`). Channeled spells never travel (Spell.cpp:3927).

use bevy::prelude::*;

use super::{PlannedEffect, TargetPlan, attributes, is_channeled, now, resolve_effects};
use crate::spell_info::{SpellInfo, SpellStore};

/// `SPELL_ATTR9_MISSILE_SPEED_IS_DELAY_IN_SEC` (SharedDefines.h:773).
const SPELL_ATTR9_MISSILE_SPEED_IS_DELAY_IN_SEC: u32 = 0x0000_0010;
/// `Spell::AddUnitTarget`: distances under 5 yd count as 5 ("@todo this is a hack").
const MIN_MISSILE_DISTANCE: f32 = 5.0;
/// The destination targets `targets::selector` handles (`TARGET_DEST_*`: caster 18,
/// target enemy 53, any 63, back 65, ally 132); `TARGET_SRC_CASTER` 22 is a source, not a
/// destination (`SpellCastTargets::HasDst`).
const DEST_TARGETS: [u32; 5] = [18, 53, 63, 65, 132];

/// Spell hits in flight, resolved by `resolve_due_hits`.
#[derive(Resource, Debug, Default)]
pub(crate) struct PendingSpellHits(Vec<PendingHit>);

#[derive(Debug)]
struct PendingHit {
    due: f64,
    caster: Entity,
    spell_id: u32,
    plan: TargetPlan,
    depth: u8,
    /// Each unit target and whether it was alive at launch (`TargetInfo::IsAlive`).
    alive_at_launch: Vec<(Entity, bool)>,
}

/// Resolve `plan` now if the spell has no travel time, else schedule each hit.
pub(super) fn launch(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    plan: TargetPlan,
    depth: u8,
) {
    // `PreprocessSpellLaunch` (Spell.cpp:8538-8541): only the caster enters combat.
    for target in super::hit_units(&plan) {
        if super::starts_combat(world, caster, target, spell) {
            crate::combat_refs::set_in_combat_with_suppressed(world, caster, target);
        }
    }
    let groups = hit_groups(world, caster, spell, plan);
    let launched = now(world);
    for (delay, plan) in groups {
        if delay <= 0.0 {
            hit(world, caster, spell, &plan, depth);
            continue;
        }
        let alive_at_launch = super::hit_units(&plan)
            .into_iter()
            .map(|unit| (unit, super::targets::is_alive(world, unit)))
            .collect();
        world
            .get_resource_or_init::<PendingSpellHits>()
            .0
            .push(PendingHit {
                due: launched + delay,
                caster,
                spell_id: spell.id,
                plan,
                depth,
                alive_at_launch,
            });
    }
}

/// `Spell::handle_delayed`: land every hit whose delay ran out.
pub(super) fn resolve_due_hits(world: &mut World) {
    let now = now(world);
    let due: Vec<PendingHit> = {
        let Some(mut pending) = world.get_resource_mut::<PendingSpellHits>() else {
            return;
        };
        if pending.0.iter().all(|hit| hit.due > now) {
            return;
        }
        let (due, waiting) = std::mem::take(&mut pending.0)
            .into_iter()
            .partition(|hit| hit.due <= now);
        pending.0 = waiting;
        due
    };
    for hit in due {
        land(world, hit);
    }
}

fn land(world: &mut World, hit: PendingHit) {
    if world.get_entity(hit.caster).is_err() {
        return;
    }
    let Some(spell) = world
        .get_resource::<SpellStore>()
        .and_then(|store| store.get(hit.spell_id))
        .cloned()
    else {
        return;
    };
    let reachable: Vec<Entity> = hit
        .alive_at_launch
        .iter()
        .filter(|&&(unit, alive)| {
            world.get_entity(unit).is_ok() && super::targets::is_alive(world, unit) == alive
        })
        .map(|&(unit, _)| unit)
        .collect();
    let plan: TargetPlan = hit
        .plan
        .into_iter()
        .map(|planned| PlannedEffect {
            units: planned
                .units
                .into_iter()
                .filter(|unit| reachable.contains(unit))
                .collect(),
            ..planned
        })
        .collect();
    self::hit(world, hit.caster, &spell, &plan, hit.depth);
}

/// `TargetInfo::PreprocessTarget` (Spell.cpp:2763-2764): each unit the spell lands on
/// enters combat with the caster, lifting the suppression of its launch reference;
/// then the effects resolve.
fn hit(world: &mut World, caster: Entity, spell: &SpellInfo, plan: &TargetPlan, depth: u8) {
    for target in super::hit_units(plan) {
        if super::starts_combat(world, caster, target, spell) {
            crate::combat_refs::set_in_combat_with(world, target, caster);
        }
    }
    resolve_effects(world, caster, spell, plan, depth);
}

/// The plan split by hit delay in seconds, earliest first. Effects without unit
/// targets (areas, summons) go with the earliest hit.
fn hit_groups(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    plan: TargetPlan,
) -> Vec<(f64, TargetPlan)> {
    let misc = spell.misc.as_ref();
    let speed = misc.map_or(0.0, |misc| misc.speed);
    let launch_delay = misc.map_or(0.0, |misc| misc.launch_delay);
    let min_duration = misc.map_or(0.0, |misc| misc.min_duration);
    let speed_is_delay = attributes(spell, 9) & SPELL_ATTR9_MISSILE_SPEED_IS_DELAY_IN_SEC != 0;
    if is_channeled(spell) || (speed <= 0.0 && launch_delay <= 0.0 && !speed_is_delay) {
        return vec![(0.0, plan)];
    }
    // `Spell::AddUnitTarget` (Spell.cpp:2508-2515): unit targets count at least 5 yd.
    let flight = |distance: f32| -> f64 {
        let travel = if speed_is_delay {
            speed.max(min_duration)
        } else if speed > 0.0 {
            (distance / speed).max(min_duration)
        } else {
            0.0
        };
        f64::from(launch_delay + travel)
    };
    let origin = position(world, caster);
    let dest = plan
        .iter()
        .filter(|planned| {
            spell.effects[planned.index]
                .implicit_targets
                .iter()
                .any(|id| DEST_TARGETS.contains(id))
        })
        .find_map(|planned| planned.dest);
    if let Some(dest) = dest {
        // `CalculateDelayMomentForDst`: one missile over the caster's exact distance.
        let distance = origin.map_or(0.0, |origin| origin.distance(dest.position));
        return vec![(flight(distance), plan)];
    }
    let mut delays: Vec<(Entity, f64)> = super::hit_units(&plan)
        .into_iter()
        .map(|unit| {
            let delay = if unit == caster {
                0.0
            } else {
                let distance = origin
                    .zip(position(world, unit))
                    .map_or(0.0, |(from, to)| from.distance(to));
                flight(distance.max(MIN_MISSILE_DISTANCE))
            };
            (unit, (delay * 1000.0).floor() / 1000.0)
        })
        .collect();
    delays.sort_by(|a, b| a.1.total_cmp(&b.1));
    let mut moments: Vec<f64> = delays.iter().map(|&(_, delay)| delay).collect();
    moments.dedup();
    if moments.is_empty() {
        moments.push(0.0);
    }
    moments
        .iter()
        .enumerate()
        .map(|(index, &moment)| {
            let group: Vec<Entity> = delays
                .iter()
                .filter(|&&(_, delay)| delay == moment)
                .map(|&(unit, _)| unit)
                .collect();
            let plan = plan
                .iter()
                .filter_map(|planned| {
                    let units: Vec<Entity> = planned
                        .units
                        .iter()
                        .copied()
                        .filter(|unit| group.contains(unit))
                        .collect();
                    let unitless = planned.units.is_empty() && index == 0;
                    (!units.is_empty() || unitless).then(|| PlannedEffect {
                        index: planned.index,
                        units,
                        dest: planned.dest,
                    })
                })
                .collect();
            (moment, plan)
        })
        .collect()
}

fn position(world: &World, unit: Entity) -> Option<Vec3> {
    world
        .get::<shared::components::Position>(unit)
        .map(|position| Vec3::new(position.x, position.y, position.z))
}

#[cfg(test)]
#[path = "missiles_tests.rs"]
mod tests;
