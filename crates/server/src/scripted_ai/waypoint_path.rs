//! `MotionMaster::MovePath` for C++ scripts: TC a352b1fa WaypointMovementGenerator
//! on MoveSplineInit/MoveSpline/Spline, for the path shapes scripts use today.
//! Unported (rejected by `move_path`): node delays/orientations, backwards paths and
//! repeating paths. Ground (gravity) units would pathfind non-exact nodes; only
//! gravity-disabled units, whose PathGenerator builds a shortcut, are supported.

use bevy::prelude::*;
use rusqlite::Connection;
use shared::components::{CreatureMotion, MovementSpeed, Position, Rotation};
use std::collections::HashMap;
use std::sync::Arc;

use super::Hook;
use super::movement::Walking;
use crate::combat::DeathState;
use crate::creature_ai::SpawnPoint;
use crate::interest::{InterestDirtyQueue, mark_interest_position_changed};

const BASE_WALK_SPEED: f32 = 2.5;
const BASE_RUN_SPEED: f32 = 7.0;
/// `DoInitialize`: `_nextMoveTime.Reset(1000)` before the first node.
const INITIALIZE_WAIT_MS: i32 = 1_000;
const SEND_NEXT_POINT_EARLY_DELTA_MS: i32 = 1_500;
/// `SplineBase::stepsPerSegment`.
const STEPS_PER_SEGMENT: u32 = 3;
/// `MoveSpline` `minimal_duration`.
const MINIMAL_DURATION_MS: i32 = 1;
/// `MoveSplineInitArgs::_checkPathLengths` minimum segment.
const MIN_SEGMENT_LENGTH: f32 = 0.1;

/// `WaypointMoveType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MoveType {
    Walk,
    Run,
    Land,
    TakeOff,
}

#[derive(Debug)]
pub(crate) struct ScriptPath {
    id: u32,
    move_type: MoveType,
    flags: u8,
    velocity: Option<f32>,
    has_node_timing: bool,
    /// Node id and WoW-space position.
    nodes: Vec<(u32, Vec3)>,
}

const FLAG_FOLLOW_BACKWARDS: u8 = 0x01;
const FLAG_EXACT_SPLINE: u8 = 0x02;

impl ScriptPath {
    fn exact_spline(&self) -> bool {
        self.flags & FLAG_EXACT_SPLINE != 0
    }
}

/// `WaypointMgr` paths (`waypoint_path` + `waypoint_path_node`).
#[derive(Resource, Default)]
pub(crate) struct ScriptPaths(HashMap<u32, Arc<ScriptPath>>);

pub(crate) fn load_script_paths(conn: &Connection) -> rusqlite::Result<ScriptPaths> {
    let mut query = conn.prepare(
        "SELECT p.PathId,p.MoveType,p.Flags,p.Velocity,n.NodeId,n.PositionX,n.PositionY,n.PositionZ,
                n.Orientation IS NOT NULL OR n.Delay>0
         FROM tdb_waypoint_path p JOIN tdb_waypoint_path_node n USING(PathId)
         ORDER BY p.PathId,n.NodeId",
    )?;
    let mut paths: HashMap<u32, ScriptPath> = HashMap::new();
    let mut rows = query.query([])?;
    while let Some(row) = rows.next()? {
        let id: u32 = row.get(0)?;
        let move_type = match row.get::<_, u8>(1)? {
            0 => MoveType::Walk,
            1 => MoveType::Run,
            2 => MoveType::Land,
            3 => MoveType::TakeOff,
            other => {
                warn!("waypoint path {id}: unknown MoveType {other}");
                continue;
            }
        };
        let path = paths.entry(id).or_insert_with(|| ScriptPath {
            id,
            move_type,
            flags: 0,
            velocity: None,
            has_node_timing: false,
            nodes: Vec::new(),
        });
        path.flags = row.get(2)?;
        path.velocity = row.get(3)?;
        path.has_node_timing |= row.get::<_, bool>(8)?;
        path.nodes.push((
            row.get(4)?,
            Vec3::new(row.get(5)?, row.get(6)?, row.get(7)?),
        ));
    }
    Ok(ScriptPaths(
        paths
            .into_iter()
            .map(|(id, path)| (id, Arc::new(path)))
            .collect(),
    ))
}

/// `SetDisableGravity(true)`: `MOVEMENTFLAG_DISABLE_GRAVITY`, so `CanFly()`.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct GravityDisabled;

/// A running non-repeating `WaypointMovementGenerator`.
#[derive(Component)]
pub(crate) struct PathMovement {
    path: Arc<ScriptPath>,
    node: usize,
    started: bool,
    wait_ms: i32,
    spline: Option<Spline>,
}

/// `MotionMaster::MovePath(pathId, false)`.
pub(crate) fn move_path(world: &mut World, unit: Entity, path_id: u32) -> bool {
    let Some(path) = world
        .get_resource::<ScriptPaths>()
        .and_then(|paths| paths.0.get(&path_id).cloned())
    else {
        error!("MovePath {path_id}: path not loaded");
        return false;
    };
    let flying = world.get::<GravityDisabled>(unit).is_some();
    if path.has_node_timing || path.flags & FLAG_FOLLOW_BACKWARDS != 0 {
        error!("MovePath {path_id}: node delay/orientation or backwards path not ported");
        return false;
    }
    if !flying && !path.exact_spline() {
        error!("MovePath {path_id}: ground pathfinding between nodes not ported");
        return false;
    }
    world.entity_mut(unit).insert(PathMovement {
        path,
        node: 0,
        started: false,
        wait_ms: INITIALIZE_WAIT_MS,
        spline: None,
    });
    true
}

/// `MoveSpline` over WoW-space points with cumulative segment times in ms.
struct Spline {
    smooth: bool,
    /// Linear: the control points. Catmull-Rom: with `InitCatmullRom`'s virtual ends.
    points: Vec<Vec3>,
    /// `Spline::lengths` from the first segment, starting at `minimal_duration`.
    times: Vec<i32>,
    elapsed_ms: i32,
    /// Finish time, earlier than the spline duration when the generator relaunches early.
    end_ms: i32,
    last_node: usize,
}

impl Spline {
    fn segments(&self) -> usize {
        self.times.len() - 1
    }

    fn control(&self, segment: usize) -> usize {
        if self.smooth { segment + 1 } else { segment }
    }

    fn evaluate(&self, segment: usize, u: f32) -> Vec3 {
        let index = self.control(segment);
        if self.smooth {
            catmull_rom(&self.points[index - 1..index + 3], u)
        } else {
            self.points[index].lerp(self.points[index + 1], u)
        }
    }

    fn segment_length(&self, segment: usize) -> f32 {
        if !self.smooth {
            let index = self.control(segment);
            return self.points[index].distance(self.points[index + 1]);
        }
        let mut length = 0.0;
        let mut previous = self.evaluate(segment, 0.0);
        for step in 1..=STEPS_PER_SEGMENT {
            let next = self.evaluate(segment, step as f32 / STEPS_PER_SEGMENT as f32);
            length += previous.distance(next);
            previous = next;
        }
        length
    }

    /// `MoveSpline::computePosition`.
    fn position(&self, time_ms: i32) -> Vec3 {
        let segment = (0..self.segments())
            .find(|&segment| time_ms < self.times[segment + 1])
            .unwrap_or(self.segments() - 1);
        let duration = (self.times[segment + 1] - self.times[segment]) as f32;
        let u = if duration > 0.0 {
            ((time_ms - self.times[segment]) as f32 / duration).min(1.0)
        } else {
            1.0
        };
        self.evaluate(segment, u)
    }
}

/// `C_Evaluate` with `s_catmullRomCoeffs`.
fn catmull_rom(p: &[Vec3], t: f32) -> Vec3 {
    let (t2, t3) = (t * t, t * t * t);
    let weights = [
        -0.5 * t3 + t2 - 0.5 * t,
        1.5 * t3 - 2.5 * t2 + 1.0,
        -1.5 * t3 + 2.0 * t2 + 0.5 * t,
        0.5 * t3 - 0.5 * t2,
    ];
    p[0] * weights[0] + p[1] * weights[1] + p[2] * weights[2] + p[3] * weights[3]
}

/// `MoveSplineInit::Launch` (+ `Validate`) for `controls`, the first of which is the
/// unit's real position; `None` when TC refuses a too-short path.
fn launch(controls: Vec<Vec3>, smooth: bool, orientation: f32, velocity: f32) -> Option<Spline> {
    let short = controls
        .windows(2)
        .any(|pair| pair[0].distance(pair[1]) < MIN_SEGMENT_LENGTH);
    if short {
        return None;
    }
    let last_node = controls.len() - 1;
    let points = if smooth {
        let first = controls[0] - Vec3::new(orientation.cos(), orientation.sin(), 0.0);
        let last = controls[last_node];
        std::iter::once(first)
            .chain(controls)
            .chain(std::iter::once(last))
            .collect()
    } else {
        controls
    };
    let mut spline = Spline {
        smooth,
        points,
        times: vec![MINIMAL_DURATION_MS],
        elapsed_ms: 0,
        end_ms: 0,
        last_node,
    };
    let velocity_inv = 1000.0 / velocity;
    let mut time = MINIMAL_DURATION_MS;
    for segment in 0..last_node {
        time = (time as f32 + spline.segment_length(segment) * velocity_inv) as i32;
        spline.times.push(time);
    }
    spline.end_ms = time;
    Some(spline)
}

/// `MoveSplineInit::Launch` velocity selection and client speed limit.
fn path_velocity(path: &ScriptPath, spawn: &SpawnPoint, walking: bool, flying: bool) -> f32 {
    let walk = match path.move_type {
        MoveType::Walk => true,
        MoveType::Run => false,
        MoveType::Land | MoveType::TakeOff => walking,
    };
    let run = spawn.speed_run * BASE_RUN_SPEED;
    let velocity = path.velocity.unwrap_or(if walk {
        spawn.speed_walk * BASE_WALK_SPEED
    } else {
        run
    });
    // Catmullrom splines need CanFly, so `Flying` covers both 50-yard limits.
    let limit = if flying { 50.0 } else { (run * 4.0).max(28.0) };
    velocity.min(limit)
}

struct Launch<'a> {
    position: Vec3,
    orientation: f32,
    spawn: &'a SpawnPoint,
    walking: bool,
    flying: bool,
}

/// `WaypointMovementGenerator::StartMove`: the next spline, or the path end.
fn start_move(movement: &mut PathMovement, launch_from: &Launch) -> Option<u32> {
    let path = movement.path.clone();
    if movement.started {
        // `ComputeNextNode` after `OnArrived`.
        if movement.node + 1 == path.nodes.len() {
            return Some(path.nodes[movement.node].0);
        }
        movement.node += 1;
    }
    movement.started = true;
    let exact = path.exact_spline();
    let targets = if exact {
        &path.nodes[movement.node..]
    } else {
        &path.nodes[movement.node..=movement.node]
    };
    let controls: Vec<Vec3> = std::iter::once(launch_from.position)
        .chain(targets.iter().map(|(_, position)| *position))
        .collect();
    let smooth = exact && controls.len() > 2 && launch_from.flying;
    let velocity = path_velocity(
        &path,
        launch_from.spawn,
        launch_from.walking,
        launch_from.flying,
    );
    let mut spline = launch(controls, smooth, launch_from.orientation, velocity);
    if let Some(spline) = &mut spline {
        spline.last_node = movement.node + spline.last_node - 1;
        let interior = movement.node != 0 && movement.node != path.nodes.len() - 1;
        if !exact
            && spline.end_ms > 2 * SEND_NEXT_POINT_EARLY_DELTA_MS
            && path.nodes.len() > 2
            && interior
        {
            spline.end_ms -= SEND_NEXT_POINT_EARLY_DELTA_MS;
        }
    }
    movement.spline = spline;
    None
}

fn wow(position: &Position) -> Vec3 {
    let (x, y, z) = crate::terrain_height::bevy_to_wow(position.x, position.y, position.z);
    Vec3::new(x, y, z)
}

fn bevy(position: Vec3) -> Position {
    let (x, y, z) = crate::terrain_height::wow_to_bevy(position.x, position.y, position.z);
    Position { x, y, z }
}

/// Advance every script path; a finished path sets home and informs the AI.
pub(crate) fn path_movement_system(world: &mut World) {
    let dt_ms = world.resource::<Time>().delta().as_millis() as i32;
    let mut ended = Vec::new();
    let mut moved = Vec::new();
    let mut changed = Vec::new();
    let mut movers = world.query_filtered::<(
        Entity,
        &mut Position,
        &mut PathMovement,
        &SpawnPoint,
        Has<Walking>,
        Has<GravityDisabled>,
        Option<&mut Rotation>,
        Option<&mut CreatureMotion>,
        Option<&mut MovementSpeed>,
    ), (
        Without<DeathState>,
        Without<super::jump_movement::JumpMovement>,
    )>();
    for (entity, mut position, mut movement, spawn, walking, flying, rotation, motion, speed) in
        movers.iter_mut(world)
    {
        // `Unit::Update`: `UpdateSplineMovement`, then the generator's `DoUpdate`.
        let before = *position;
        if let Some(spline) = &mut movement.spline {
            spline.elapsed_ms = (spline.elapsed_ms + dt_ms).min(spline.end_ms);
            *position = bevy(spline.position(spline.elapsed_ms));
            if spline.elapsed_ms == spline.end_ms {
                movement.node = spline.last_node;
                movement.spline = None;
            }
        } else if movement.wait_ms > 0 {
            movement.wait_ms -= dt_ms;
        }
        if movement.spline.is_none() && movement.wait_ms <= 0 {
            let launch_from = Launch {
                position: wow(&position),
                orientation: rotation
                    .as_ref()
                    .map_or(0.0, |rotation| rotation.y - std::f32::consts::FRAC_PI_2),
                spawn,
                walking,
                flying,
            };
            if let Some(node_id) = start_move(&mut movement, &launch_from) {
                ended.push((entity, node_id, movement.path.id, *position));
            }
        }
        let gait = match (&movement.spline, walking) {
            (None, _) => CreatureMotion::Still,
            (Some(_), true) => CreatureMotion::Walk,
            (Some(_), false) => CreatureMotion::Run,
        };
        if *position != before {
            moved.push(entity);
        }
        let facing = crate::creature_ai::facing_along(position.x - before.x, position.z - before.z);
        let mut dirty = motion.is_some_and(|mut motion| motion.set_if_neq(gait));
        if let Some(mut speed) = speed
            && gait != CreatureMotion::Still
        {
            let velocity = path_velocity(&movement.path, spawn, walking, flying);
            dirty |= speed.set_if_neq(MovementSpeed(velocity));
        }
        if let (Some(mut rotation), Some(facing)) = (rotation, facing)
            && rotation.y != facing
        {
            rotation.y = facing;
            dirty = true;
        }
        if dirty {
            changed.push(entity);
        }
    }
    if let Some(queue) = world.get_resource::<lightyear::prelude::ReplicationDirtyQueue>() {
        for &entity in &changed {
            queue.mark(entity);
        }
    }
    if let Some(queue) = world.get_resource::<InterestDirtyQueue>() {
        for entity in moved {
            mark_interest_position_changed(Some(queue), entity);
        }
    }
    for (entity, node_id, path_id, home) in ended {
        // `SetHomePosition` at the last node, then `WaypointPathEnded`.
        world
            .entity_mut(entity)
            .remove::<PathMovement>()
            .insert(super::movement::ScriptedPath { home });
        super::dispatch(world, entity, Hook::WaypointPathEnded { node_id, path_id });
    }
}
