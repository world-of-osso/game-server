//! `MotionMaster::MovePath` for C++ scripts: TC a352b1fa WaypointMovementGenerator
//! on MoveSplineInit/MoveSpline/Spline, for the path shapes scripts use today.
//! Unported (rejected by `move_path`): node delays/orientations, backwards paths and
//! repeating paths. Ground non-exact nodes use TC smooth-surface baked controls;
//! gravity-disabled units retain the source PathGenerator shortcut.

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

#[cfg(test)]
#[path = "waypoint_path_tests.rs"]
mod tests;

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
#[cfg_attr(test, derive(Clone))]
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
    deactivated: bool,
    finalized: bool,
    active_goal: Option<crate::navigation::Goal>,
    wait_ms: i32,
    spline: Option<Spline>,
}

impl PathMovement {
    fn query_goal(&self) -> crate::navigation::Goal {
        let node = self.node + usize::from(self.started);
        crate::navigation::Goal::Waypoint(self.path.id, node)
    }

    /// An active chase/home generator preempts this default-slot waypoint.
    fn deactivate(&mut self) {
        if let Some(spline) = self.spline.take() {
            if self.path.exact_spline() {
                // Exact controls correspond one-for-one with authored nodes.
                self.node += spline.segment_at(spline.elapsed_ms);
            }
            self.started = false;
        }
        self.deactivated = true;
    }

    /// Initialize a new active-slot identity exactly once, regardless of which
    /// system first deactivated the lower default generator.
    pub(crate) fn preempt_for(&mut self, goal: crate::navigation::Goal) -> bool {
        self.deactivate();
        let initializing = self.active_goal != Some(goal);
        self.active_goal = Some(goal);
        initializing
    }

    /// WaypointMovementGenerator::DoReset: stop the old spline, then relaunch
    /// the current target from the owner's real position, not stale controls.
    fn reactivate(&mut self) -> bool {
        if !self.deactivated {
            return false;
        }
        self.deactivated = false;
        self.active_goal = None;
        if self.wait_ms <= 0 {
            self.wait_ms = MINIMAL_DURATION_MS;
        }
        true
    }
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
    let requires_ground_navigation = !flying && !path.exact_spline();
    if requires_ground_navigation && !world.contains_resource::<crate::navigation::Navigation>() {
        error!("MovePath {path_id}: ground path requires baked navigation");
        return false;
    }
    let previous = world
        .get::<PathMovement>(unit)
        .map(PathMovement::query_goal);
    if let Some(goal) = previous
        && let Some(mut nav) = world.get_resource_mut::<crate::navigation::Navigation>()
    {
        nav.cancel_query(unit, goal);
    }
    world.entity_mut(unit).insert(PathMovement {
        path,
        node: 0,
        started: false,
        deactivated: false,
        finalized: false,
        active_goal: None,
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

    fn segment_at(&self, time_ms: i32) -> usize {
        (0..self.segments())
            .find(|&segment| time_ms < self.times[segment + 1])
            .unwrap_or(self.segments() - 1)
    }

    /// `MoveSpline::computePosition`.
    fn position(&self, time_ms: i32) -> Vec3 {
        let segment = self.segment_at(time_ms);
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
    // TC _checkPathLengths exempts the first crossing from the actual owner
    // position. Two-point paths are permitted, including an already-reached node.
    let short = find_short_interior_segment(&controls).is_some();
    if controls.len() < 2 || short {
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

fn path_walk_mode(path: &ScriptPath, walking: bool) -> bool {
    match path.move_type {
        MoveType::Walk => true,
        MoveType::Run => false,
        MoveType::Land | MoveType::TakeOff => walking,
    }
}

/// `MoveSplineInit::Launch` velocity selection and client speed limit.
fn path_velocity(path: &ScriptPath, spawn: &SpawnPoint, walking: bool, flying: bool) -> f32 {
    let walk = path_walk_mode(path, walking);
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
    entity: Entity,
    map: crate::maps::InMap,
    position: Vec3,
    orientation: f32,
    spawn: &'a SpawnPoint,
    walking: bool,
    flying: bool,
}

/// `WaypointMovementGenerator::StartMove`: the next spline, or the path end.
fn start_move(
    movement: &mut PathMovement,
    launch_from: &Launch,
    navigation: Option<&mut crate::navigation::Navigation>,
    ground: Option<&mut crate::ground::GroundMap>,
) -> Result<Option<u32>, String> {
    let path = movement.path.clone();
    // Compute the next target without committing generator state while pending.
    let node = if movement.started {
        if movement.node + 1 == path.nodes.len() {
            return Ok(Some(path.nodes[movement.node].0));
        }
        movement.node + 1
    } else {
        movement.node
    };
    let exact = path.exact_spline();
    let Some(controls) = query_path_controls(&path, node, launch_from, navigation, ground)? else {
        return Ok(None);
    };
    let smooth = exact && controls.len() > 2 && launch_from.flying;
    let velocity = path_velocity(
        &path,
        launch_from.spawn,
        launch_from.walking,
        launch_from.flying,
    );
    let short_segment = find_short_interior_segment(&controls);
    let node_id = path.nodes[node].0;
    let mut spline = launch(controls, smooth, launch_from.orientation, velocity)
        .ok_or_else(|| format!("spline controls rejected at authored node {node_id}: short segment {short_segment:?}"))?;
    spline.last_node = if exact {
        node + spline.last_node - 1
    } else {
        node
    };
    let interior = node != 0 && node != path.nodes.len() - 1;
    if !exact
        && spline.end_ms > 2 * SEND_NEXT_POINT_EARLY_DELTA_MS
        && path.nodes.len() > 2
        && interior
    {
        spline.end_ms -= SEND_NEXT_POINT_EARLY_DELTA_MS;
    }
    movement.node = node;
    movement.started = true;
    movement.spline = Some(spline);
    Ok(None)
}

fn find_short_interior_segment(controls: &[Vec3]) -> Option<(usize, f32)> {
    controls
        .windows(2)
        .enumerate()
        .skip(1)
        .find_map(|(index, pair)| {
            let distance = pair[0].distance(pair[1]);
            (distance < MIN_SEGMENT_LENGTH).then_some((index, distance))
        })
}

/// TC CreateSingularPointPath: owner position, then calculated path after its first
/// point. Never reinterpret Detour corners as authored waypoint node indices.
fn query_path_controls(
    path: &ScriptPath,
    node: usize,
    launch_from: &Launch,
    navigation: Option<&mut crate::navigation::Navigation>,
    ground: Option<&mut crate::ground::GroundMap>,
) -> Result<Option<Vec<Vec3>>, String> {
    if path.exact_spline() || launch_from.flying {
        let targets = if path.exact_spline() {
            &path.nodes[node..]
        } else {
            &path.nodes[node..=node]
        };
        return Ok(Some(
            std::iter::once(launch_from.position)
                .chain(targets.iter().map(|(_, position)| *position))
                .collect(),
        ));
    }
    let navigation = navigation.ok_or("ground path has no navigation resource")?;
    let ground = ground.ok_or("ground path requires collision geometry")?;
    let route = query_ground_waypoints(
        navigation,
        ground,
        launch_from,
        path.nodes[node].1,
        crate::navigation::Goal::Waypoint(path.id, node),
    )?;
    Ok(route.map(|route| std::iter::once(launch_from.position).chain(route).collect()))
}

fn query_ground_waypoints(
    navigation: &mut crate::navigation::Navigation,
    ground: &mut crate::ground::GroundMap,
    launch_from: &Launch,
    target: Vec3,
    goal: crate::navigation::Goal,
) -> Result<Option<Vec<Vec3>>, String> {
    let target = bevy(target);
    let from = bevy(launch_from.position);
    let Some(result) = navigation.query_deferred(
        launch_from.entity,
        launch_from.map.map_id,
        goal,
        ::navigation::worker::Query::SmoothPath {
            from: Vec3::new(from.x, from.y, from.z),
            to: Vec3::new(target.x, target.y, target.z),
        },
    ) else {
        return Ok(None);
    };
    navigation.cancel_query(launch_from.entity, goal);
    let route = match result? {
        Some(::navigation::worker::Output::Path(route)) => route,
        None => return Err("ground path map has no navigation bake".into()),
        Some(_) => return Err("ground waypoint returned a non-path result".into()),
    };
    route
        .into_iter()
        .skip(1)
        .map(|point| {
            let position = sample_script_ground(
                Some(&mut *ground),
                launch_from.map,
                Position {
                    x: point.x,
                    y: point.y,
                    z: point.z,
                },
            )?;
            Ok(wow(&position))
        })
        .collect::<Result<Vec<_>, String>>()
        .map(Some)
}

/// Exact creature collision support, shared by generated controls and moving feet.
fn sample_script_ground(
    ground: Option<&mut crate::ground::GroundMap>,
    map: crate::maps::InMap,
    mut position: Position,
) -> Result<Position, String> {
    let ground = ground.ok_or("ground waypoint requires collision geometry")?;
    let point = Vec3::new(position.x, position.y, position.z);
    let crate::ground::GroundProbe::Supported(height) = ground.creature_ground_in(map, point)
    else {
        return Err(format!(
            "ground waypoint lacks collision support at {point:?}"
        ));
    };
    position.y = height;
    Ok(position)
}

fn wow(position: &Position) -> Vec3 {
    let (x, y, z) = crate::terrain_height::bevy_to_wow(position.x, position.y, position.z);
    Vec3::new(x, y, z)
}

fn bevy(position: Vec3) -> Position {
    let (x, y, z) = crate::terrain_height::wow_to_bevy(position.x, position.y, position.z);
    Position { x, y, z }
}

fn advance_path_generator(
    movement: &mut PathMovement,
    position: &mut Position,
    launch_from: &Launch,
    dt_ms: i32,
    navigation: Option<&mut crate::navigation::Navigation>,
    mut ground: Option<&mut crate::ground::GroundMap>,
) -> Result<Option<u32>, String> {
    let ground_path = !launch_from.flying && !movement.path.exact_spline();
    if let Some(spline) = &mut movement.spline {
        spline.elapsed_ms = (spline.elapsed_ms + dt_ms).min(spline.end_ms);
        let candidate = bevy(spline.position(spline.elapsed_ms));
        *position = if ground_path {
            sample_script_ground(ground.as_deref_mut(), launch_from.map, candidate)?
        } else {
            candidate
        };
        if spline.elapsed_ms == spline.end_ms {
            movement.node = spline.last_node;
            movement.spline = None;
        }
    } else if movement.wait_ms > 0 {
        movement.wait_ms -= dt_ms;
    }
    if movement.spline.is_none() && movement.wait_ms <= 0 {
        let current = Launch {
            position: wow(position),
            ..*launch_from
        };
        return start_move(movement, &current, navigation, ground);
    }
    Ok(None)
}

/// Advance every script path; a finished path sets home and informs the AI.
pub(crate) fn path_movement_system(world: &mut World) {
    let dt_ms = world.resource::<Time>().delta().as_millis() as i32;
    // Exclusive system: keep the navigation cache out of World's mutable query borrow.
    let mut navigation = world.remove_resource::<crate::navigation::Navigation>();
    let mut ground = world.remove_resource::<crate::ground::GroundMap>();
    let mut ended = Vec::new();
    let mut retired = Vec::new();
    let mut failed = Vec::new();
    let mut moved = Vec::new();
    let mut changed = Vec::new();
    let mut walking_modes = Vec::new();
    let mut movers = world.query_filtered::<(
        Entity,
        &mut Position,
        &mut PathMovement,
        &SpawnPoint,
        &crate::maps::InMap,
        Option<&crate::creature_ai::CreatureState>,
        Has<Walking>,
        Has<GravityDisabled>,
        Option<&mut Rotation>,
        Option<&mut CreatureMotion>,
        Option<&mut MovementSpeed>,
    ), (
        Without<DeathState>,
        Without<super::jump_movement::JumpMovement>,
        Without<crate::scripted_ai::fall_movement::FallMovement>,
        Without<crate::spell_cast::charge::ChargeMovement>,
    )>();
    for (
        entity,
        mut position,
        mut movement,
        spawn,
        map,
        state,
        walking,
        flying,
        rotation,
        motion,
        speed,
    ) in movers.iter_mut(world)
    {
        if movement.finalized {
            retired.push(entity);
            continue;
        }
        // MotionMaster updates only its top generator: active chase/home wins
        // over this default-slot MovePath; route generation is not changed.
        let reset = match state {
            Some(
                crate::creature_ai::CreatureState::Pursuing { .. }
                | crate::creature_ai::CreatureState::Returning,
            ) => {
                if let Some(nav) = navigation.as_mut() {
                    nav.cancel_query(entity, movement.query_goal());
                }
                movement.deactivate();
                continue;
            }
            _ => movement.reactivate(),
        };
        // A reset top generator receives diff0 on its initialization update.
        let update_ms = if reset { 0 } else { dt_ms };
        // `Unit::Update`: `UpdateSplineMovement`, then the generator's `DoUpdate`.
        let before = *position;
        let launch_from = Launch {
            entity,
            map: *map,
            position: wow(&position),
            orientation: rotation
                .as_ref()
                .map_or(0.0, |rotation| rotation.y - std::f32::consts::FRAC_PI_2),
            spawn,
            walking,
            flying,
        };
        match advance_path_generator(
            &mut movement,
            &mut position,
            &launch_from,
            update_ms,
            navigation.as_mut(),
            ground.as_mut(),
        ) {
            Ok(Some(node_id)) => ended.push((entity, node_id, movement.path.id, *position)),
            Ok(None) => {}
            Err(error) => {
                failed.push((entity, movement.path.id, error));
                movement.spline = None;
            }
        }
        let path_walk = path_walk_mode(&movement.path, walking);
        if movement.spline.is_some() && path_walk != walking {
            walking_modes.push((entity, path_walk));
        }
        let gait = match (&movement.spline, path_walk) {
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
    if let Some(navigation) = navigation {
        world.insert_resource(navigation);
    }
    if let Some(ground) = ground {
        world.insert_resource(ground);
    }
    for entity in retired {
        world.entity_mut(entity).remove::<PathMovement>();
    }
    for (entity, walking) in walking_modes {
        super::movement::set_walk(world, entity, walking);
    }
    for (entity, path_id, error) in failed {
        error!("MovePath {path_id} for {entity:?}: {error}; stopped without arrival");
        world.entity_mut(entity).remove::<PathMovement>();
        if world
            .get_mut::<CreatureMotion>(entity)
            .is_some_and(|mut motion| motion.set_if_neq(CreatureMotion::Still))
        {
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
        // Keep finalized spline ownership through the callback/handoff. A new
        // active generator gets diff0 even when this was the final path step;
        // retire it at the next movement update, without a second end callback.
        world.get_mut::<PathMovement>(entity).unwrap().finalized = true;
        world
            .entity_mut(entity)
            .insert(super::movement::ScriptedPath { home });
        super::dispatch(world, entity, Hook::WaypointPathEnded { node_id, path_id });
    }
}
