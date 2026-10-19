//! Lazy baked navigation and per-creature route progress. No runtime asset extraction.
use bevy::ecs::entity::EntityHashMap;
use bevy::prelude::*;
use shared::components::Position;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

const REPATH_DISTANCE: f32 = 1.0;
const TELEPORT_DISTANCE: f32 = 2.0;

mod deferred;
#[cfg(test)]
mod fixture_queries;
mod requests;
use deferred::PendingQuery;
use requests::{PendingRoute, publish_navigation_results};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Goal {
    Chase(u64),
    Follow(u64),
    Hazard(u64),
    Approach(u64),
    FearCollision,
    FearPath,
    Point,
    Home,
    Waypoint(u32, usize),
}

#[derive(Clone, Copy)]
pub struct MoveRequest {
    pub map_id: u32,
    pub goal: Goal,
    pub from: Position,
    pub to: Position,
    pub distance: f32,
    pub reach: f32,
}

pub enum Step {
    Pending,
    Unbaked,
    Moved { position: Position, reached: bool },
    Blocked,
}

#[derive(Debug)]
struct Route {
    map_id: u32,
    goal: Goal,
    destination: Vec3,
    last_target: Vec3,
    expected_from: Vec3,
    path: Result<Corners, String>,
    reach: f32,
}

#[derive(Debug)]
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
    routes: EntityHashMap<Route>,
    worker: navigation::worker::Worker,
    pending: EntityHashMap<PendingRoute>,
    queries: EntityHashMap<PendingQuery>,
    unbaked: HashSet<u32>,
    next_id: u64,
    #[cfg(test)]
    applied: Vec<(Entity, u64)>,
}

impl Navigation {
    pub fn new(dir: &Path) -> Self {
        Self {
            dir: dir.to_path_buf(),
            routes: EntityHashMap::default(),
            worker: navigation::worker::Worker::new(dir.to_path_buf()),
            pending: EntityHashMap::default(),
            queries: EntityHashMap::default(),
            unbaked: HashSet::new(),
            next_id: 0,
            #[cfg(test)]
            applied: Vec::new(),
        }
    }

    pub(crate) fn route_destination(&self, entity: Entity, goal: Goal) -> Option<Vec3> {
        self.routes
            .get(&entity)
            .filter(|route| route.goal == goal)
            .map(|route| route.destination)
    }

    /// Log the borrowed pre-step cache without changing route progress.
    pub(crate) fn trace_cached_route(&self, entity: Entity, simulation_secs: f64) {
        debug!(target: "game_server::chase_trace",
            actor = entity.to_bits(), simulation_secs,
            route = ?self.routes.get(&entity), "tank cached chase route");
    }

    /// Drop an interrupted chase corridor before it can resume from stale corners.
    pub(crate) fn cancel_route(&mut self, entity: Entity) {
        let had_route = self.routes.remove(&entity).is_some();
        let had_request = self.pending.remove(&entity).is_some();
        if (had_route || had_request) && !self.query_pending(entity) {
            self.worker.cancel(entity.to_bits());
        }
    }

    pub fn advance(&mut self, entity: Entity, request: MoveRequest) -> Step {
        self.invalidate_moved_route(entity, &request);
        if self.unbaked.contains(&request.map_id) {
            self.cancel_route(entity);
            return Step::Unbaked;
        }
        let current = self
            .routes
            .get(&entity)
            .is_some_and(|route| route.reusable(&request));
        if !current {
            self.request_route(entity, request);
        }
        self.advance_published_route(entity, request, !current)
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
                if last || (in_reach_leg && within_reach(at, target, reach)) {
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

/// The existing reach is a3Dsphere; chase admission and route arrival share this test.
pub(crate) fn within_reach(from: Vec3, target: Vec3, reach: f32) -> bool {
    from.distance_squared(target) <= reach * reach
}

/// Ground travel to the actual victim's3Dsphere, not its projected nav point.
/// Called only on the final corridor leg, never across an intervening obstacle.
pub(crate) fn distance_to_reach(from: Vec3, corner: Vec3, target: Vec3, reach: f32) -> f32 {
    let offset = from - target;
    let delta = corner - from;
    let distance = horizontal(delta);
    let c = offset.length_squared() - reach * reach;
    if within_reach(from, target, reach) {
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

#[derive(Resource)]
struct NavigationPublicationInstalled;

fn install_publication(app: &mut App) {
    if app
        .world()
        .contains_resource::<NavigationPublicationInstalled>()
    {
        return;
    }
    app.insert_resource(NavigationPublicationInstalled);
    app.add_systems(
        FixedFirst,
        (prune_routes, publish_navigation_results).chain(),
    );
}

#[cfg(test)]
#[derive(Resource)]
struct NavigationFixtureDrainInstalled;

#[cfg(test)]
fn drain_fixture_navigation(mut navigation: ResMut<Navigation>) {
    navigation.drain_until_idle();
}

#[cfg(test)]
pub(crate) fn install_fixture(app: &mut App, navigation: Navigation) {
    install_publication(app);
    app.insert_resource(navigation);
    if !app
        .world()
        .contains_resource::<NavigationFixtureDrainInstalled>()
    {
        app.insert_resource(NavigationFixtureDrainInstalled);
        app.add_systems(First, (prune_routes, drain_fixture_navigation).chain());
    }
}

#[cfg(test)]
pub(crate) fn drain_fixture(app: &mut App) {
    if app.world().contains_resource::<Navigation>() {
        install_publication(app);
        app.world_mut()
            .resource_mut::<Navigation>()
            .drain_until_idle();
    }
}

pub struct NavigationPlugin;
impl Plugin for NavigationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostStartup,
            open_navigation.after(crate::ground::open_ground_map),
        );
        install_publication(app);
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

#[cfg(all(test, target_os = "linux"))]
#[path = "navigation_blocking_io_tests.rs"]
pub(crate) mod blocking_io_tests;

#[cfg(all(test, target_os = "linux"))]
#[path = "navigation_request_tests.rs"]
mod request_tests;

fn prune_routes(
    mut nav: ResMut<Navigation>,
    actors: Query<(Option<&crate::maps::InMap>, Option<&Position>)>,
) {
    deferred::prune_queries(&mut nav, &actors);
    let obsolete: Vec<_> = nav
        .pending
        .iter()
        .filter_map(|(entity, pending)| {
            // Position belongs to every route owner; CreatureState describes one
            // AI driver, not navigation ownership. Map metadata remains optional.
            let Ok((map, Some(position))) = actors.get(*entity) else {
                return Some(*entity);
            };
            let wrong_map = map.is_some_and(|map| map.map_id != pending.request.map_id);
            let moved = !pending.done
                && horizontal(point(*position) - point(pending.request.from)) > TELEPORT_DISTANCE;
            (wrong_map || moved).then_some(*entity)
        })
        .collect();
    for entity in obsolete {
        nav.cancel_route(entity);
    }
}
