//! Cage of Carnage starting-area enforcement (TC arena_cage_of_carnage.cpp,
//! TDB 2025_10_04_00_world.sql): the custom entrance areatriggers cast the Arena
//! Starting Area Marker on entry and remove it on exit; from 5s after the start
//! and every 5s after, marked players without Warning receive it, and Warning's
//! removal while still marked casts Warning Teleport to their team's exit.
use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::*;
use rusqlite::Connection;
use shared::battleground::{BgPhase, BgTeam};
use shared::components::{Health, Player, Position};

use crate::maps::{InMap, TeleportOptions, WorldLocation, store::AreaTriggerShape};

const MAP: u32 = 2759;
pub(crate) const STARTING_AREA_MARKER: u32 = 228_212;
pub(crate) const WARNING: u32 = 1_214_676;
pub(crate) const WARNING_TELEPORT: u32 = 371_319;
const WARNING_INTERVAL_SECS: f64 = 5.0;
/// TC CageOfCarnage::Positions; ALLIANCE (team A, AllianceStartLoc) is Purple.
const PURPLE_WARNING_TELEPORT: (Vec3, f32) = (Vec3::new(442.311, 343.167, -34.1166), 1.745_329_3);
const GOLD_WARNING_TELEPORT: (Vec3, f32) = (Vec3::new(443.167, 439.243, -42.6562), 4.590_215_7);

#[derive(Clone)]
struct EntranceTrigger {
    center: Vec3,
    shape: AreaTriggerShape,
}

#[derive(Resource, Default)]
pub(crate) struct Catalog {
    triggers: Vec<EntranceTrigger>,
}

impl Catalog {
    /// Static map-2759 box spawns whose template's only action casts the marker on
    /// any unit (ActionType 0 CAST, TargetType 0 USER_ANY).
    pub(crate) fn load(db: &Connection) -> rusqlite::Result<Self> {
        let triggers: Vec<EntranceTrigger> = db
            .prepare(
                "SELECT a.PosX,a.PosY,a.PosZ,a.Orientation,p.Shape,p.ShapeData0,p.ShapeData1,
                        p.ShapeData2,t.ActionType,t.ActionParam,t.TargetType
                 FROM tdb_areatrigger a
                 JOIN tdb_areatrigger_create_properties p
                   ON p.Id=a.AreaTriggerCreatePropertiesId AND p.IsCustom=a.IsCustom
                 JOIN tdb_areatrigger_template_actions t
                   ON t.AreaTriggerId=p.AreaTriggerId AND t.IsCustom=p.IsAreatriggerCustom
                 WHERE a.MapId=?1 ORDER BY a.SpawnId",
            )?
            .query_map([MAP], |row| {
                let action: (u32, u32, u32) = (row.get(8)?, row.get(9)?, row.get(10)?);
                if row.get::<_, u32>(4)? != 1 || action != (0, STARTING_AREA_MARKER, 0) {
                    return Err(rusqlite::Error::InvalidQuery);
                }
                Ok(EntranceTrigger {
                    center: Vec3::new(row.get(0)?, row.get(1)?, row.get(2)?),
                    shape: AreaTriggerShape::Box {
                        half_length: row.get(5)?,
                        half_width: row.get(6)?,
                        half_height: row.get(7)?,
                        yaw: row.get(3)?,
                    },
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        if triggers.is_empty() {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(Self { triggers })
    }
}

#[derive(Default)]
struct Clock {
    marked: BTreeSet<Entity>,
    next_warning: Option<f64>,
}

#[derive(Resource, Default)]
pub(crate) struct Timers(BTreeMap<InMap, Clock>);

pub(super) fn update(world: &mut World, now: f64) {
    let maps: BTreeMap<_, _> = world
        .resource::<super::PvpServerState>()
        .battleground_matches
        .values()
        .filter_map(|matched| {
            let map = matched.map.filter(|map| map.map_id == MAP)?;
            let arena = matched.arena.as_ref()?;
            let running = matched.instance.phase == BgPhase::InProgress;
            Some((map, running.then_some(arena.started).flatten()))
        })
        .collect();
    if maps.is_empty() && !world.contains_resource::<Timers>() {
        return;
    }
    world.resource_scope(|world, mut timers: Mut<Timers>| {
        timers.0.retain(|map, _| maps.contains_key(map));
        for (map, started) in maps {
            let clock = timers.0.entry(map).or_default();
            update_presence(world, map, clock);
            match started {
                Some(start) => update_warnings(world, map, now - start, clock),
                None => clock.next_warning = None,
            }
        }
    });
}

/// TC AreaTrigger::DoActions/UndoActions for a static spawn: the unit casts the
/// marker on itself on entry; leaving removes it whoever cast it.
fn update_presence(world: &mut World, map: InMap, clock: &mut Clock) {
    let triggers = world.resource::<Catalog>().triggers.clone();
    let inside: BTreeSet<_> = players_on(world, map)
        .into_iter()
        .filter(|&player| {
            let point = crate::game_objects::wow_position(
                world
                    .get::<Position>(player)
                    .expect("arena player position"),
            );
            triggers
                .iter()
                .any(|trigger| trigger.shape.contains(trigger.center, point))
        })
        .collect();
    for &player in inside.difference(&clock.marked) {
        crate::spell_cast::cast_triggered(world, player, STARTING_AREA_MARKER, Some(player), 0);
    }
    for &player in clock.marked.difference(&inside) {
        if world.get_entity(player).is_ok() {
            crate::auras::remove_auras(
                world,
                player,
                crate::auras::RemoveReason::Cancelled,
                |aura| aura.spell_id == STARTING_AREA_MARKER,
            );
        }
    }
    clock.marked = inside;
}

/// TC OnStart schedules the first check 5s after the gates and repeats every 5s.
fn update_warnings(world: &mut World, map: InMap, elapsed: f64, clock: &mut Clock) {
    let next = clock.next_warning.get_or_insert(WARNING_INTERVAL_SECS);
    if elapsed < *next {
        return;
    }
    *next += WARNING_INTERVAL_SECS;
    for player in players_on(world, map) {
        if crate::auras::has_aura(world, player, STARTING_AREA_MARKER)
            && !crate::auras::has_aura(world, player, WARNING)
        {
            crate::spell_cast::cast_triggered(world, player, WARNING, Some(player), 0);
        }
    }
}

fn players_on(world: &mut World, map: InMap) -> Vec<Entity> {
    world
        .query_filtered::<(Entity, &InMap, &Health), With<Player>>()
        .iter(world)
        .filter(|(_, copy, health)| **copy == map && health.current > 0.0)
        .map(|(entity, ..)| entity)
        .collect()
}

/// TC spell_undermine_arena_warning_countdown (1214676 effect0 remove).
pub(crate) fn warning_removed(
    world: &mut World,
    player: Entity,
    _aura: &crate::auras::AuraInstance,
    _reason: crate::auras::RemoveReason,
) {
    let in_cage = world
        .get::<InMap>(player)
        .is_some_and(|map| map.map_id == MAP);
    if in_cage
        && world.get::<Player>(player).is_some()
        && crate::auras::has_aura(world, player, STARTING_AREA_MARKER)
    {
        crate::spell_cast::cast_triggered(world, player, WARNING_TELEPORT, Some(player), 1);
    }
}

/// TC spell_undermine_arena_warning_teleport (371319 effect0 hit): Alliance to
/// the Purple exit, otherwise Gold. Enigma Crucible binds 371319 to its own script.
pub(crate) fn warning_teleport(world: &mut World, _caster: Entity, target: Entity, _index: u8) {
    if world
        .get::<InMap>(target)
        .is_none_or(|map| map.map_id != MAP)
    {
        warn!("Warning Teleport 371319 outside Cage of Carnage has no implemented script");
        return;
    }
    let Some(team) = world
        .get::<super::PlayerPvpState>(target)
        .and_then(|state| state.battleground.as_ref())
        .map(|battleground| battleground.team)
    else {
        return;
    };
    let (position, orientation) = match team {
        BgTeam::A => PURPLE_WARNING_TELEPORT,
        BgTeam::B => GOLD_WARNING_TELEPORT,
    };
    let destination = WorldLocation {
        map_id: MAP,
        position,
        orientation,
    };
    crate::maps::teleport_to(world, target, destination, TeleportOptions::default());
}

#[cfg(test)]
#[path = "pvp_arena_cage_tests.rs"]
mod tests;
