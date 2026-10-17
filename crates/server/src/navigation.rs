//! Lazy baked navigation and per-creature route progress. No runtime asset extraction.
use bevy::ecs::entity::EntityHashMap;
use bevy::prelude::*;
use shared::components::Position;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const REPATH_DISTANCE: f32 = 1.0;
const TELEPORT_DISTANCE: f32 = 2.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Goal {
    Chase(u64),
    Follow(u64),
    Hazard(u64),
    Point,
    Home,
    Waypoint(u32, usize),
}

pub struct MoveRequest {
    pub map_id: u32,
    pub goal: Goal,
    pub from: Position,
    pub to: Position,
    pub distance: f32,
    pub reach: f32,
}

pub enum Step {
    Unbaked,
    Moved { position: Position, reached: bool },
    Blocked,
}

struct Route {
    map_id: u32,
    goal: Goal,
    destination: Vec3,
    expected_from: Vec3,
    path: Result<Corners, String>,
}

pub(crate) struct Corners {
    points: Vec<Vec3>,
    next: usize,
    terminal_leg: usize,
}

impl Corners {
    pub(crate) fn new(points: Vec<Vec3>) -> Self {
        let terminal_leg = terminal_leg(&points);
        Self {
            points,
            next: 0,
            terminal_leg,
        }
    }
}

#[derive(Resource)]
pub struct Navigation {
    dir: PathBuf,
    maps: Mutex<HashMap<u32, Option<navigation::NavigationMap>>>,
    routes: EntityHashMap<Route>,
}

impl Navigation {
    pub fn new(dir: &Path) -> Self {
        Self {
            dir: dir.to_path_buf(),
            maps: Mutex::new(HashMap::new()),
            routes: EntityHashMap::default(),
        }
    }

    /// None means an entirely unbaked map, never a missing tile on a baked map.
    pub(crate) fn query_path(
        &self,
        map_id: u32,
        from: Vec3,
        to: Vec3,
    ) -> Result<Option<Vec<Vec3>>, String> {
        self.query_map(map_id, |map| map.path(from, to))
    }

    /// Only normal ground C++ script waypoints select TC smooth-surface controls.
    pub(crate) fn query_waypoint_path(
        &self,
        map_id: u32,
        from: Vec3,
        to: Vec3,
    ) -> Result<Option<Vec<Vec3>>, String> {
        self.query_map(map_id, |map| map.smooth_waypoint_path(from, to))
    }

    fn query_map<T>(
        &self,
        map_id: u32,
        query: impl FnOnce(&mut navigation::NavigationMap) -> Result<T, String>,
    ) -> Result<Option<T>, String> {
        let mut maps = self.maps.lock().unwrap();
        if !maps.contains_key(&map_id) {
            let dir = self.dir.join(map_id.to_string());
            let baked = match std::fs::read_dir(&dir) {
                Ok(entries) => {
                    let mut found = false;
                    for entry in entries {
                        let entry = entry.map_err(|err| format!("{}: {err}", dir.display()))?;
                        found |= entry.path().extension().is_some_and(|ext| ext == "nav");
                    }
                    found
                }
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => false,
                Err(err) => return Err(format!("{}: {err}", dir.display())),
            };
            let map = if baked {
                Some(navigation::NavigationMap::open(&dir)?)
            } else {
                error!(
                    "Map {map_id} has no navigation bake in {}; no baked route available",
                    dir.display()
                );
                None
            };
            maps.insert(map_id, map);
        }
        match maps.get_mut(&map_id).unwrap() {
            Some(map) => query(map).map(Some),
            None => Ok(None),
        }
    }

    pub(crate) fn first_collision(
        &self,
        map_id: u32,
        from: Vec3,
        to: Vec3,
    ) -> Result<Option<Vec3>, String> {
        // query_path initializes the same lazy map and never creates a direct route.
        if self.query_path(map_id, from, from)?.is_none() {
            return Ok(None);
        }
        let mut maps = self.maps.lock().unwrap();
        maps.get_mut(&map_id)
            .unwrap()
            .as_mut()
            .unwrap()
            .first_collision(from, to)
            .map(Some)
    }

    pub(crate) fn route_destination(&self, entity: Entity, goal: Goal) -> Option<Vec3> {
        self.routes
            .get(&entity)
            .filter(|route| route.goal == goal)
            .map(|route| route.destination)
    }

    /// Drop an interrupted chase corridor before it can resume from stale corners.
    pub(crate) fn cancel_route(&mut self, entity: Entity) {
        self.routes.remove(&entity);
    }

    pub fn advance(&mut self, entity: Entity, request: MoveRequest) -> Step {
        let from = point(request.from);
        let to = point(request.to);
        let current = self
            .routes
            .get(&entity)
            .is_some_and(|route| route.reusable(&request));
        if !current {
            let Some(route) = self.query_route(entity, &request) else {
                self.routes.remove(&entity);
                return Step::Unbaked;
            };
            self.routes.insert(entity, route);
        }
        let route = self.routes.get_mut(&entity).unwrap();
        let Ok(corners) = &mut route.path else {
            return Step::Blocked;
        };
        let (next, reached) = corners.advance(from, request.distance, request.reach, to);
        route.expected_from = next;
        Step::Moved {
            position: position(next),
            reached,
        }
    }
    fn query_route(&self, entity: Entity, request: &MoveRequest) -> Option<Route> {
        let from = point(request.from);
        let to = point(request.to);
        let path = match self.query_path(request.map_id, from, to) {
            Ok(None) => return None,
            Ok(Some(points)) => Ok(Corners::new(points)),
            Err(err) => {
                error!(
                    "Creature {entity} navigation {:?} on map {} failed: {err}; no direct movement",
                    request.goal, request.map_id
                );
                Err(err)
            }
        };
        Some(Route {
            map_id: request.map_id,
            goal: request.goal,
            destination: to,
            expected_from: from,
            path,
        })
    }
}

impl Route {
    fn reusable(&self, request: &MoveRequest) -> bool {
        let same_goal = self.map_id == request.map_id && self.goal == request.goal;
        let target_is_current = if matches!(request.goal, Goal::Follow(_)) {
            self.destination == point(request.to)
        } else {
            self.destination.distance(point(request.to)) <= REPATH_DISTANCE
        };
        let position_is_current =
            horizontal(self.expected_from - point(request.from)) <= TELEPORT_DISTANCE;
        same_goal && target_is_current && position_is_current
    }
}

impl Corners {
    pub(crate) fn advance(
        &mut self,
        mut at: Vec3,
        mut budget: f32,
        reach: f32,
        target: Vec3,
    ) -> (Vec3, bool) {
        while let Some(&corner) = self.points.get(self.next) {
            let delta = corner - at;
            let distance = horizontal(delta);
            let last = self.next + 1 == self.points.len();
            let in_reach_leg = self.next >= self.terminal_leg && reach > 0.0;
            let remaining = if in_reach_leg {
                distance_to_reach(at, corner, target, reach)
            } else {
                distance
            };
            if remaining <= budget {
                if distance > 0.0001 {
                    at += delta * (remaining / distance);
                } else {
                    at.y = corner.y;
                }
                if last || (in_reach_leg && horizontal(at - target) <= reach + 0.0001) {
                    return (at, true);
                }
                budget -= remaining;
                self.next += 1;
            } else {
                at += delta * (budget / distance);
                return (at, false);
            }
        }
        (at, true)
    }
}

/// Polygon crossings on the final funnel segment are not new turns. Begin the
/// reach test at that segment's first crossing, not just at the final waypoint.
fn terminal_leg(points: &[Vec3]) -> usize {
    if points.len() < 2 {
        return 0;
    }
    let end = points[points.len() - 1];
    let mut first = points.len() - 2;
    while first > 0 {
        let a = Vec2::new(end.x - points[first].x, end.z - points[first].z).normalize_or_zero();
        let b = Vec2::new(
            points[first].x - points[first - 1].x,
            points[first].z - points[first - 1].z,
        )
        .normalize_or_zero();
        if a.dot(b) < 0.9999 {
            break;
        }
        first -= 1;
    }
    first + 1
}

/// Stop on the actual victim's reach circle, not two yards from its projected nav point.
/// Called only on the final corridor leg, never across an intervening obstacle.
fn distance_to_reach(from: Vec3, corner: Vec3, target: Vec3, reach: f32) -> f32 {
    let offset = Vec2::new(from.x - target.x, from.z - target.z);
    let delta = Vec2::new(corner.x - from.x, corner.z - from.z);
    let distance = delta.length();
    let c = offset.length_squared() - reach * reach;
    if c <= 0.0 {
        return 0.0;
    }
    let a = delta.length_squared();
    let b = 2.0 * offset.dot(delta);
    let discriminant = b * b - 4.0 * a * c;
    if a <= 0.000001 || discriminant < 0.0 {
        return distance;
    }
    let fraction = (-b - discriminant.sqrt()) / (2.0 * a);
    if !(0.0..=1.0).contains(&fraction) {
        return distance;
    }
    distance * fraction
}

fn horizontal(delta: Vec3) -> f32 {
    delta.x.hypot(delta.z)
}
fn point(p: Position) -> Vec3 {
    Vec3::new(p.x, p.y, p.z)
}
fn position(p: Vec3) -> Position {
    Position {
        x: p.x,
        y: p.y,
        z: p.z,
    }
}

pub struct NavigationPlugin;
impl Plugin for NavigationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostStartup,
            open_navigation.after(crate::ground::open_ground_map),
        );
        app.add_systems(FixedUpdate, prune_routes);
    }
}

fn open_navigation(mut commands: Commands) {
    commands.insert_resource(Navigation::new(
        &crate::ground::ground_data_dir().join("navmesh"),
    ));
}

#[cfg(test)]
#[path = "navigation_tests.rs"]
mod tests;

fn prune_routes(
    mut nav: ResMut<Navigation>,
    creatures: Query<Entity, With<crate::creature_ai::CreatureState>>,
) {
    nav.routes.retain(|entity, _| creatures.contains(*entity));
}
