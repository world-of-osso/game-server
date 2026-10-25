//! TC a352b1fa arena_dalaran_sewers.cpp: authentic30/5/30s waterfall,
//! 1500ms center knockback, six-second pipe flush and sourced pipe trigger shapes.
use crate::maps::{InMap, store::AreaTriggerShape};
use bevy::prelude::*;
use rusqlite::Connection;
use shared::components::{Health, Npc, Player, Position};
use shared::protocol::{GameObjectAnimation, GameObjectCollision, GameObjectInfo};
use std::collections::{BTreeMap, BTreeSet};

const MAP: u32 = 617;
const SPOUT_ENTRY: u32 = 28567;
const WATERFALL: u32 = 191877;
const WATER_COLLISION: u32 = 194395;
const DEMONIC_CIRCLE: u32 = 48018;
const FLUSH: u32 = 57405;
const PIPE_SEARCH_AURA: u32 = 96539;
const PET_TELEPORT: u32 = 254013;
const CENTER_PUSH: u32 = 58873;
const WARNING_AT: f64 = 30.0;
const COLLISION_AT: f64 = 35.0;
const CYCLE_SECS: f64 = 65.0;
const CENTER_INTERVAL_SECS: f64 = 1.5;
const PIPE_FLUSH_AT: f64 = 6.0;
const PIPE_SPEED_XY: f32 = 30.0;
const PIPE_SPEED_Z: f32 = 19.0;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WaterSpout {
    Pipe,
    Center,
}

#[derive(Clone)]
struct PipeTrigger {
    id: u32,
    center: Vec3,
    shape: AreaTriggerShape,
}
#[derive(Resource, Default)]
pub(crate) struct Catalog {
    spouts: BTreeMap<u32, WaterSpout>,
    triggers: Vec<PipeTrigger>,
}
impl Catalog {
    pub(crate) fn load(db: &Connection) -> rusqlite::Result<Self> {
        let spouts = db
            .prepare("SELECT guid,string_id FROM arena_spout WHERE map_id=617 AND entry=28567")?
            .query_map([], |row| {
                let kind = match row.get::<_, String>(1)?.as_str() {
                    "arena_dalaran_sewers_water_spout_pipe" => WaterSpout::Pipe,
                    "arena_dalaran_sewers_water_spout_center" => WaterSpout::Center,
                    _ => return Err(rusqlite::Error::InvalidQuery),
                };
                Ok((row.get(0)?, kind))
            })?
            .collect::<rusqlite::Result<_>>()?;
        let triggers = db.prepare("SELECT a.ID,a.Pos_0,a.Pos_1,a.Pos_2,a.ShapeType,a.Radius,a.Box_length,a.Box_width,a.Box_height,a.Box_yaw FROM area_trigger a JOIN tdb_areatrigger_scripts s ON s.entry=a.ID WHERE a.ContinentID=617 AND s.ScriptName='at_ds_pipe_knockback'")?.query_map([], |row| {
            let shape = AreaTriggerShape::from_db2(row.get(4)?, row.get(5)?, [row.get(6)?, row.get(7)?, row.get(8)?, row.get(9)?]).ok_or(rusqlite::Error::InvalidQuery)?;
            Ok(PipeTrigger { id: row.get(0)?, center: Vec3::new(row.get(1)?, row.get(2)?, row.get(3)?), shape })
        })?.collect::<rusqlite::Result<_>>()?;
        Ok(Self { spouts, triggers })
    }
}

pub(crate) fn tag_spout(mut actor: EntityWorldMut, guid: u32) {
    if actor.get::<InMap>().is_none_or(|map| map.map_id != MAP)
        || actor
            .get::<Npc>()
            .is_none_or(|npc| npc.template_id != SPOUT_ENTRY)
    {
        return;
    }
    let kind = actor
        .world()
        .resource::<Catalog>()
        .spouts
        .get(&guid)
        .copied();
    match kind {
        Some(kind) => {
            actor.insert(kind);
        }
        None => error!("Dalaran spout guid{guid} has no imported source string identity"),
    }
}

#[derive(Default)]
struct Clock {
    opened: bool,
    flushed: bool,
    water_on: bool,
    next_center_push: Option<f64>,
    pipe_presence: BTreeSet<(Entity, u32)>,
}
#[derive(Resource, Default)]
pub(crate) struct Timers(BTreeMap<InMap, Clock>);

pub(super) fn reset_map(world: &mut World, map: InMap) {
    world.resource_mut::<Timers>().0.remove(&map);
}

pub(super) fn update(world: &mut World, now: f64) {
    let maps: BTreeMap<_, _> = world
        .resource::<super::PvpServerState>()
        .battleground_matches
        .values()
        .filter_map(|matched| {
            let map = matched.map.filter(|map| map.map_id == MAP)?;
            let arena = matched.arena.as_ref()?;
            Some((map, (matched.instance.phase, arena.started)))
        })
        .collect();
    if maps.is_empty() && !world.contains_resource::<Timers>() {
        return;
    }
    world.resource_scope(|world, mut timers: Mut<Timers>| {
        let expired: Vec<_> = timers
            .0
            .keys()
            .copied()
            .filter(|map| !maps.contains_key(map))
            .collect();
        for map in expired {
            stop_hazards(world, map);
            timers.0.remove(&map);
        }
        for (map, (phase, started)) in maps {
            if phase == shared::battleground::BgPhase::InProgress {
                let start = started.expect("running arena start");
                update_running(world, map, now - start, timers.0.entry(map).or_default());
            } else {
                stop_hazards(world, map);
                timers.0.remove(&map);
            }
        }
    });
}

fn update_running(world: &mut World, map: InMap, elapsed: f64, clock: &mut Clock) {
    if !clock.opened {
        for player in players_on(world, map) {
            clear_circle(world, player);
        }
        clock.opened = true;
    }
    if elapsed >= PIPE_FLUSH_AT && !clock.flushed {
        for spout in spouts_on(world, map, WaterSpout::Pipe) {
            crate::spell_cast::cast_triggered(world, spout, FLUSH, None, 0);
            crate::spell_cast::cast_triggered(world, spout, PIPE_SEARCH_AURA, Some(spout), 0);
        }
        clock.flushed = true;
    }
    let cycle_time = elapsed % CYCLE_SECS;
    let warning = cycle_time >= WARNING_AT;
    let active = cycle_time >= COLLISION_AT;
    set_water_object(world, map, WATERFALL, warning);
    set_water_object(world, map, WATER_COLLISION, active);
    update_center_pushes(world, map, elapsed, active, clock);
    update_pipe_presence(world, map, clock);
}

fn update_center_pushes(
    world: &mut World,
    map: InMap,
    elapsed: f64,
    active: bool,
    clock: &mut Clock,
) {
    if active && !clock.water_on {
        clock.next_center_push = Some(elapsed + CENTER_INTERVAL_SECS);
    }
    if !active {
        clock.next_center_push = None;
    }
    clock.water_on = active;
    if clock.next_center_push.is_some_and(|next| elapsed >= next) {
        for spout in spouts_on(world, map, WaterSpout::Center) {
            crate::spell_cast::cast_triggered(world, spout, CENTER_PUSH, Some(spout), 0);
        }
        clock.next_center_push = Some(elapsed + CENTER_INTERVAL_SECS);
    }
}

fn set_water_object(world: &mut World, map: InMap, entry: u32, active: bool) {
    // Source spawns start ACTIVE0 (off). UseDoor toggles them to READY1 (on).
    // Display8592/FDID252201 is static: only the visual waterfall has door clips.
    let desired_animation = (entry == WATERFALL).then_some(GameObjectAnimation {
        animation_id: if active { 147 } else { 148 },
        looping: false,
    });
    let desired_collision = GameObjectCollision(active);
    let changes: Vec<_> = world
        .query::<(
            Entity,
            &InMap,
            &GameObjectInfo,
            Option<&GameObjectAnimation>,
            Option<&GameObjectCollision>,
        )>()
        .iter(world)
        .filter(|(_, copy, info, animation, collision)| {
            **copy == map
                && info.entry == entry
                && (*animation != desired_animation.as_ref()
                    || *collision != Some(&desired_collision))
        })
        .map(|(entity, ..)| entity)
        .collect();
    for entity in changes {
        let mut object = world.entity_mut(entity);
        object.insert(desired_collision);
        if let Some(animation) = desired_animation {
            object.insert(animation);
        } else {
            object.remove::<GameObjectAnimation>();
        }
    }
}

fn spouts_on(world: &mut World, map: InMap, kind: WaterSpout) -> Vec<Entity> {
    world
        .query::<(Entity, &InMap, &WaterSpout)>()
        .iter(world)
        .filter(|(_, copy, source)| **copy == map && **source == kind)
        .map(|(entity, ..)| entity)
        .collect()
}
fn players_on(world: &mut World, map: InMap) -> Vec<Entity> {
    world
        .query_filtered::<(Entity, &InMap, &Health), With<Player>>()
        .iter(world)
        .filter(|(_, copy, health)| **copy == map && health.current > 0.0)
        .map(|(entity, ..)| entity)
        .collect()
}
fn clear_circle(world: &mut World, player: Entity) {
    crate::auras::remove_auras(
        world,
        player,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == DEMONIC_CIRCLE,
    );
}

fn update_pipe_presence(world: &mut World, map: InMap, clock: &mut Clock) {
    let triggers = world.resource::<Catalog>().triggers.clone();
    let current: BTreeSet<_> = players_on(world, map)
        .into_iter()
        .flat_map(|player| {
            let point = crate::game_objects::wow_position(
                world
                    .get::<Position>(player)
                    .expect("arena player position"),
            );
            triggers
                .iter()
                .filter(move |trigger| trigger.shape.contains(trigger.center, point))
                .map(move |trigger| (player, trigger.id))
        })
        .collect();
    for &(player, _) in current.symmetric_difference(&clock.pipe_presence) {
        if world.get::<InMap>(player) == Some(&map) {
            clear_circle(world, player);
            crate::spell_cast::cast_triggered(world, player, PET_TELEPORT, Some(player), 0);
        }
    }
    clock.pipe_presence = current;
}

fn stop_hazards(world: &mut World, map: InMap) {
    set_water_object(world, map, WATERFALL, false);
    set_water_object(world, map, WATER_COLLISION, false);
    for spout in spouts_on(world, map, WaterSpout::Pipe) {
        crate::auras::remove_auras(
            world,
            spout,
            crate::auras::RemoveReason::Cancelled,
            |aura| aura.spell_id == PIPE_SEARCH_AURA,
        );
    }
}

/// TC source96538s1 names57405; use the authored value, not an invented search spell.
pub(crate) fn cast_pipe_search(
    world: &mut World,
    ctx: &crate::spell_cast::effects::EffectCtx,
    effect: &crate::spell_info::SpellEffect,
) {
    let spell =
        crate::spell_cast::effects::modded_base_points(world, ctx.caster, ctx.spell, effect) as u32;
    crate::spell_cast::cast_triggered(world, ctx.caster, spell, None, ctx.depth + 1);
}

/// Source57405 force-casts server-side61698: its pinned script pushes the caster30/19.
/// 61698 has no local DB2 row, so no synthetic SpellInfo/visual payload is constructed.
pub(crate) fn push_pipe_player(world: &mut World, player: Entity, spout: Entity) {
    let Some(origin) = crate::spell_cast::targets::position(world, spout) else {
        error!("Dalaran pipe caster has no position");
        return;
    };
    if let Err(error) = crate::knockback::launch(world, player, origin, PIPE_SPEED_XY, PIPE_SPEED_Z)
    {
        error!("Dalaran pipe push on {player:?}: {error}");
    }
}

#[cfg(test)]
#[path = "pvp_arena_dalaran_tests.rs"]
mod tests;
