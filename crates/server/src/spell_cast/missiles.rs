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

use std::sync::Arc;

use bevy::prelude::*;

use super::reflect::{self, Reflection};
use super::{PlannedEffect, TargetPlan, attributes, is_channeled, now, resolve_effects};
use crate::spell_info::SpellInfo;

/// `SPELL_ATTR9_MISSILE_SPEED_IS_DELAY_IN_SEC` (SharedDefines.h:773).
const SPELL_ATTR9_MISSILE_SPEED_IS_DELAY_IN_SEC: u32 = 0x0000_0010;
/// `Spell::AddUnitTarget`: distances under 5 yd count as 5 ("@todo this is a hack").
const MIN_MISSILE_DISTANCE: f32 = 5.0;
/// Audited destination targets: caster18/front47, enemy53/any63/back65/cursor87/ally132.
/// `TARGET_SRC_CASTER`22 is a source, not `SpellCastTargets::HasDst`.
const DEST_TARGETS: &[u32] = &[18, 47, 53, 63, 65, 87, 132];

/// `SPELL_ATTR4_BOUNCY_CHAIN_MISSILES` (SharedDefines.h:602): "Hits area targets over
/// time instead of all at once".
const SPELL_ATTR4_BOUNCY_CHAIN_MISSILES: u32 = 0x0004_0000;

/// Spell hits in flight, resolved by `resolve_due_hits`.
#[derive(Resource, Debug, Default)]
pub(crate) struct PendingSpellHits(Vec<PendingHit>);

#[derive(Debug)]
struct PendingHit {
    due: f64,
    caster: Entity,
    spell: Arc<SpellInfo>,
    depth: u8,
    landing: Landing,
}

/// What a hit in flight does when it lands.
#[derive(Debug)]
enum Landing {
    /// The plan's units, each with whether it was alive at launch (`TargetInfo::IsAlive`).
    Units {
        plan: TargetPlan,
        alive_at_launch: Vec<(Entity, bool)>,
    },
    /// A spell `reflector` reflected: `plan` resolves on the caster, if the reflector is
    /// still there in the alive state it had at launch.
    Reflected {
        reflector: Entity,
        alive_at_launch: bool,
        plan: TargetPlan,
    },
    /// The missile reaches `reflector`: its reflect aura procs.
    ReflectProc { reflector: Entity },
}

/// Resolve launch-mode effects, then `plan` now if the spell has no travel time,
/// else schedule each hit.
pub(super) fn launch(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    plan: TargetPlan,
    depth: u8,
) {
    // All successful ordinary/triggered casts reach this once, before flight/hit.
    crate::scripted_ai::on_spell_cast(world, caster, spell.id);
    let reflections: Vec<(Entity, Reflection)> = super::hit_units(&plan)
        .into_iter()
        .filter_map(|unit| Some((unit, reflect::roll(world, caster, spell, unit)?)))
        .collect();
    // `PreprocessSpellLaunch` (Spell.cpp:8538-8541): only the caster enters combat.
    for target in super::hit_units(&plan) {
        if super::starts_combat(world, caster, target, spell) {
            crate::combat_refs::set_in_combat_with_suppressed(world, caster, target);
        }
    }
    for &(unit, reflection) in &reflections {
        reflect::log_miss(world, caster, spell, unit, reflection);
    }
    let charge = match super::charge::launch(world, caster, spell, &plan) {
        Ok(charge) => charge,
        Err(error) => {
            error!("Spell{} charge launch: {error}", spell.id);
            return;
        }
    };
    let (mut delays, destination_delay) = hit_delays(world, caster, spell, &plan);
    if let Some((target, duration_ms)) = charge {
        for (unit, delay) in &mut delays {
            if *unit == target {
                *delay = duration_ms;
            }
        }
    }
    if spell.id == shared::battleground::AZERITE_GATHERING_SPELL {
        debug!(caster = ?caster, misc = ?spell.misc, delays = ?delays, "Azerite launch delays");
    }
    let launched = now(world);
    let (plan, reflected) = split_reflected(plan, &reflections, caster);
    super::resolve_launch_effects(world, caster, spell, &plan, depth);
    for (_, reflected) in &reflected {
        super::resolve_launch_effects(world, caster, spell, reflected, depth);
    }
    let mut snapshot = None;
    for (delay, landing) in landings(world, plan, reflected, &delays, destination_delay) {
        if delay == 0 {
            land_now(world, caster, spell, depth, landing);
            continue;
        }
        world
            .get_resource_or_init::<PendingSpellHits>()
            .0
            .push(PendingHit {
                due: launched + delay as f64 / 1000.0,
                caster,
                spell: Arc::clone(snapshot.get_or_insert_with(|| Arc::new(spell.clone()))),
                depth,
                landing,
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
        if world.get_entity(hit.caster).is_err() {
            continue;
        }
        land_now(world, hit.caster, &hit.spell, hit.depth, hit.landing);
    }
}

fn land_now(world: &mut World, caster: Entity, spell: &SpellInfo, depth: u8, landing: Landing) {
    match landing {
        Landing::Units {
            plan,
            alive_at_launch,
        } => {
            let reachable: Vec<Entity> = alive_at_launch
                .iter()
                .filter(|&&(unit, alive)| reachable(world, unit, alive))
                .map(|&(unit, _)| unit)
                .collect();
            let plan = plan
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
            hit(world, caster, spell, &plan, depth);
        }
        Landing::Reflected {
            reflector,
            alive_at_launch,
            plan,
        } => {
            if !reachable(world, reflector, alive_at_launch) {
                return;
            }
            // `PreprocessTarget` engages the target, which is still the reflector.
            if super::starts_combat(world, caster, reflector, spell) {
                crate::combat_refs::set_in_combat_with(world, reflector, caster);
            }
            resolve_effects(world, caster, spell, &plan, depth);
        }
        Landing::ReflectProc { reflector } => reflect::proc_reflector(world, caster, reflector),
    }
}

/// `TargetInfo::PreprocessTarget` / `DoTargetSpellHit` (Spell.cpp:2746-2748, 2807): the
/// unit is still in the world and alive as it was at launch.
fn reachable(world: &World, unit: Entity, alive_at_launch: bool) -> bool {
    world.get_entity(unit).is_ok() && super::targets::is_alive(world, unit) == alive_at_launch
}

/// `TargetInfo::PreprocessTarget` (Spell.cpp:2763-2764): each unit the spell lands on
/// enters combat with the caster, lifting the suppression of its launch reference;
/// then the effects resolve.
fn hit(world: &mut World, caster: Entity, spell: &SpellInfo, plan: &TargetPlan, depth: u8) {
    if spell.id == shared::battleground::AZERITE_GATHERING_SPELL {
        debug!(caster = ?caster, plan = ?plan, "Azerite landing plan");
    }
    for target in super::hit_units(plan) {
        if super::starts_combat(world, caster, target, spell) {
            crate::combat_refs::set_in_combat_with(world, target, caster);
        }
    }
    resolve_effects(world, caster, spell, plan, depth);
}

/// The plan without its reflected and deflected units, and per reflector the effects
/// that would have hit it, turned on the caster (with the reflector's chain multiplier).
fn split_reflected(
    plan: TargetPlan,
    reflections: &[(Entity, Reflection)],
    caster: Entity,
) -> (TargetPlan, Vec<(Entity, TargetPlan)>) {
    let reflected = reflections
        .iter()
        .filter(|(_, reflection)| *reflection == Reflection::Reflect)
        .map(|&(reflector, _)| {
            let turned = plan
                .iter()
                .filter(|planned| planned.units.contains(&reflector))
                .map(|planned| PlannedEffect {
                    index: planned.index,
                    units: vec![caster],
                    dest: planned.dest,
                    chain_multipliers: vec![(caster, planned.chain_multiplier(reflector))],
                    blocked_units: Vec::new(),
                    prevent_hit_script: planned.prevent_hit_script,
                })
                .collect();
            (reflector, turned)
        })
        .collect();
    // An effect left without its units drops out, rather than resolving as unitless.
    let plan = plan
        .into_iter()
        .filter_map(|planned| {
            let had_units = !planned.units.is_empty();
            let units: Vec<Entity> = planned
                .units
                .iter()
                .copied()
                .filter(|unit| !reflections.iter().any(|(reflector, _)| reflector == unit))
                .collect();
            (units.len() > 0 || !had_units).then(|| PlannedEffect { units, ..planned })
        })
        .collect();
    (plan, reflected)
}

/// Every landing with its delay in ms, earliest first: the plan split by hit delay
/// (effects without units use the destination's delay, otherwise the earliest hit),
/// and per reflector its aura proc at its delay and the reflected spell at 1.5 × it
/// ("Increase time interval for reflected spells by 1.5", Spell.cpp:2543-2544).
fn landings(
    world: &World,
    plan: TargetPlan,
    reflected: Vec<(Entity, TargetPlan)>,
    delays: &[(Entity, u64)],
    destination_delay: Option<u64>,
) -> Vec<(u64, Landing)> {
    let delay_of = |unit: Entity| {
        delays
            .iter()
            .find(|(delayed, _)| *delayed == unit)
            .map_or(0, |(_, delay)| *delay)
    };
    let units = super::hit_units(&plan);
    let mut moments: Vec<u64> = units.iter().map(|&unit| delay_of(unit)).collect();
    moments.sort_unstable();
    let unitless_moment = destination_delay.unwrap_or_else(|| {
        moments
            .first()
            .copied()
            .unwrap_or_else(|| delays.iter().map(|(_, delay)| *delay).min().unwrap_or(0))
    });
    if plan.iter().any(|planned| planned.units.is_empty()) {
        moments.push(unitless_moment);
    }
    moments.sort_unstable();
    moments.dedup();
    let mut landings: Vec<(u64, Landing)> = moments
        .iter()
        .map(|&moment| {
            let plan: TargetPlan = plan
                .iter()
                .filter_map(|planned| {
                    let units: Vec<Entity> = planned
                        .units
                        .iter()
                        .copied()
                        .filter(|&unit| delay_of(unit) == moment)
                        .collect();
                    let unitless = planned.units.is_empty() && moment == unitless_moment;
                    (!units.is_empty() || unitless).then(|| PlannedEffect {
                        units,
                        ..planned.clone()
                    })
                })
                .collect();
            let alive_at_launch = super::hit_units(&plan)
                .into_iter()
                .map(|unit| (unit, super::targets::is_alive(world, unit)))
                .collect();
            (
                moment,
                Landing::Units {
                    plan,
                    alive_at_launch,
                },
            )
        })
        .collect();
    for (reflector, plan) in reflected {
        let delay = delay_of(reflector);
        landings.push((delay, Landing::ReflectProc { reflector }));
        landings.push((
            delay + (delay >> 1),
            Landing::Reflected {
                reflector,
                alive_at_launch: super::targets::is_alive(world, reflector),
                plan,
            },
        ));
    }
    landings.sort_by_key(|(delay, _)| *delay);
    landings
}

/// Unit hit delays plus the independent destination delay, retained even without units.
/// Each unit target's hit delay in ms (`Spell::AddUnitTarget`, Spell.cpp:2486-2518),
/// in plan order. Without travel time every delay is 0. A destination spell is one
/// missile over the caster's exact distance (`CalculateDelayMomentForDst`) for every
/// non-caster unit. Otherwise a unit is `LaunchDelay` plus the flight from the caster, except with
/// `SPELL_ATTR4_BOUNCY_CHAIN_MISSILES`: a unit after the first that shares an effect
/// with an earlier one is hit the flight from the latest such unit after it, without
/// another `LaunchDelay`. The caster's own entry is 0.
fn hit_delays(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    plan: &TargetPlan,
) -> (Vec<(Entity, u64)>, Option<u64>) {
    let units = super::hit_units(plan);
    let misc = spell.misc.as_ref();
    let speed = misc.map_or(0.0, |misc| misc.speed);
    let launch_delay = misc.map_or(0.0, |misc| misc.launch_delay);
    let min_duration = misc.map_or(0.0, |misc| misc.min_duration);
    let speed_is_delay = attributes(spell, 9) & SPELL_ATTR9_MISSILE_SPEED_IS_DELAY_IN_SEC != 0;
    if is_channeled(spell) || (speed <= 0.0 && launch_delay <= 0.0 && !speed_is_delay) {
        return (units.into_iter().map(|unit| (unit, 0)).collect(), None);
    }
    let travel = |distance: f32| -> f32 {
        if speed_is_delay {
            speed.max(min_duration)
        } else if speed > 0.0 {
            (distance / speed).max(min_duration)
        } else {
            0.0
        }
    };
    let to_ms = |secs: f32| (f64::from(secs) * 1000.0).floor() as u64;
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
        let distance = origin.map_or(0.0, |origin| origin.distance(dest.position));
        let delay = to_ms(launch_delay + travel(distance));
        return (
            // TC Spell.cpp:2486–2521,4096–4138: self hits precede the destination.
            units
                .into_iter()
                .map(|unit| (unit, if unit == caster { 0 } else { delay }))
                .collect(),
            Some(delay),
        );
    }
    let bouncy = attributes(spell, 4) & SPELL_ATTR4_BOUNCY_CHAIN_MISSILES != 0;
    let effect_mask = |unit: Entity| -> u64 {
        plan.iter()
            .filter(|planned| planned.units.contains(&unit))
            .fold(0, |mask, planned| mask | 1 << planned.index)
    };
    let mut delays: Vec<(Entity, u64)> = Vec::with_capacity(units.len());
    for unit in units {
        if unit == caster {
            delays.push((unit, 0));
            continue;
        }
        let mask = effect_mask(unit);
        let previous = bouncy
            .then(|| {
                delays
                    .iter()
                    .rev()
                    .find(|&&(earlier, _)| effect_mask(earlier) & mask != 0)
                    .copied()
            })
            .flatten();
        let (source, start, launch) = match previous {
            Some((earlier, delay)) => (position(world, earlier), delay, 0.0),
            None => (origin, 0, launch_delay),
        };
        let distance = source
            .zip(position(world, unit))
            .map_or(0.0, |(from, to)| from.distance(to));
        let flight = launch + travel(distance.max(MIN_MISSILE_DISTANCE));
        delays.push((unit, start + to_ms(flight)));
    }
    (delays, None)
}

fn position(world: &World, unit: Entity) -> Option<Vec3> {
    world
        .get::<shared::components::Position>(unit)
        .map(|position| Vec3::new(position.x, position.y, position.z))
}

#[cfg(test)]
#[path = "missiles_tests.rs"]
mod tests;
