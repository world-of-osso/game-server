//! Route a held follower along the owner's recorded safe turns.
use super::{owner_trail, positioning};
use crate::maps::InMap;
use crate::navigation::{Goal, MoveRequest, Navigation};
use ::navigation::worker::{Output, Query};
use bevy::prelude::*;
use shared::components::Position;

const HOLD_LOG_INTERVAL_SECONDS: f64 = 10.0;
const ARRIVAL_DISTANCE: f32 = 0.75;

#[derive(Component)]
struct TrailFollow {
    owner: Entity,
    blockers: Vec<Entity>,
    last_hold_log: Option<f64>,
    candidates: Vec<Vec3>,
    destination: Option<(usize, Vec3)>,
}

pub(super) fn active(world: &World, unit: Entity, owner: Entity) -> bool {
    world
        .get::<TrailFollow>(unit)
        .is_some_and(|state| state.owner == owner)
}

pub(super) fn advance(
    world: &mut World,
    unit: Entity,
    owner: Entity,
    blockers: Vec<Entity>,
) -> bool {
    let now = world.resource::<Time>().elapsed_secs_f64();
    let Some(map) = world.get::<InMap>(unit).copied() else {
        return false;
    };
    let Some(from) = world.get::<Position>(unit).copied() else {
        return false;
    };
    let origin = Vec3::new(from.x, from.y, from.z);
    if !active(world, unit, owner) {
        let candidates = owner_trail::points(world, owner, map);
        world.entity_mut(unit).insert(TrailFollow {
            owner,
            blockers,
            last_hold_log: None,
            candidates,
            destination: None,
        });
    }
    let blocked = world
        .get::<TrailFollow>(unit)
        .expect("trail state")
        .blockers
        .iter()
        .any(|&watcher| positioning::is_unpulled_hostile(world, unit, watcher));
    if !blocked {
        clear(world, unit);
        return true;
    }
    let destination = world
        .get::<TrailFollow>(unit)
        .expect("trail state")
        .destination;
    if let Some((index, to)) = destination {
        if origin.distance(to) <= ARRIVAL_DISTANCE {
            let mut state = world.get_mut::<TrailFollow>(unit).expect("trail state");
            state.destination = None;
            state.candidates.drain(..=index);
            if state.candidates.is_empty() {
                clear(world, unit);
                return false;
            }
            return true;
        }
        let goal = Goal::OwnerTrail(owner.to_bits());
        super::hazards::advance_escape(
            world,
            unit,
            map.map_id,
            from,
            Position {
                x: to.x,
                y: to.y,
                z: to.z,
            },
            goal,
        );
        if world
            .get::<Position>(unit)
            .is_some_and(|after| *after == from)
        {
            world
                .get_mut::<TrailFollow>(unit)
                .expect("trail state")
                .destination = None;
            world.resource_mut::<Navigation>().cancel_route(unit);
            log_hold(world, unit, owner, now, "trail step refused");
        }
        return true;
    }
    let mut candidates = world
        .get::<TrailFollow>(unit)
        .expect("trail state")
        .candidates
        .clone();
    if candidates.is_empty() {
        candidates = owner_trail::points(world, owner, map);
        world
            .get_mut::<TrailFollow>(unit)
            .expect("trail state")
            .candidates = candidates.clone();
    }
    if candidates.is_empty() {
        log_hold(world, unit, owner, now, "owner trail unavailable");
        return true;
    }
    let Some(mut nav) = world.get_resource_mut::<Navigation>() else {
        log_hold(world, unit, owner, now, "navigation unavailable");
        return true;
    };
    let goal = Goal::OwnerTrail(owner.to_bits());
    let result = nav.query_deferred(
        unit,
        map.map_id,
        goal,
        Query::Candidates {
            from: origin,
            targets: candidates.clone(),
        },
    );
    let paths = match result {
        None => return true,
        Some(Ok(Some(Output::Candidates(paths)))) => paths,
        Some(other) => {
            let reason = format!("owner trail navigation failed: {other:?}");
            log_hold(world, unit, owner, now, &reason);
            return true;
        }
    };
    let Some((index, to, path)) = choose_route(world, unit, origin, candidates, paths) else {
        world
            .get_mut::<TrailFollow>(unit)
            .expect("trail state")
            .candidates
            .clear();
        world.resource_mut::<Navigation>().cancel_query(unit, goal);
        log_hold(world, unit, owner, now, "no safe owner trail route");
        return true;
    };
    debug!(?unit, ?owner, ?origin, destination = ?to, "Follower retracing owner trail");
    world.resource_mut::<Navigation>().adopt_candidate_route(
        unit,
        MoveRequest {
            map_id: map.map_id,
            goal,
            from,
            to: Position {
                x: to.x,
                y: to.y,
                z: to.z,
            },
            distance: 0.0,
            reach: 0.0,
        },
        path,
    );
    let mut state = world.get_mut::<TrailFollow>(unit).expect("trail state");
    state.destination = Some((index, to));
    true
}

fn choose_route(
    world: &World,
    unit: Entity,
    origin: Vec3,
    candidates: Vec<Vec3>,
    paths: Vec<Result<Vec<Vec3>, String>>,
) -> Option<(usize, Vec3, Vec<Vec3>)> {
    let exclusions = world.get::<positioning::CombatMovementExclusions>(unit)?;
    let los = world.get_resource::<crate::line_of_sight::LineOfSight>();
    candidates
        .into_iter()
        .zip(paths)
        .enumerate()
        .rev()
        .find_map(|(index, (to, path))| {
            let path = path.ok()?;
            if origin.distance(to) <= ARRIVAL_DISTANCE {
                return None;
            }
            let projected_start = path
                .first()
                .is_some_and(|point| (point.x - origin.x).hypot(point.z - origin.z) < 0.001);
            let mut previous = origin;
            let safe = path
                .iter()
                .copied()
                .skip(usize::from(projected_start))
                .all(|point| {
                    let permitted = exclusions.permits(los, previous, point);
                    previous = point;
                    permitted
                });
            safe.then_some((index, to, path))
        })
}

fn log_hold(world: &mut World, unit: Entity, owner: Entity, now: f64, reason: &str) {
    let mut state = world.get_mut::<TrailFollow>(unit).expect("trail state");
    if state
        .last_hold_log
        .is_none_or(|last| now - last >= HOLD_LOG_INTERVAL_SECONDS)
    {
        warn!(
            ?unit,
            ?owner,
            reason,
            "Follower holds while avoiding idle hostiles"
        );
        state.last_hold_log = Some(now);
    }
}

#[cfg(test)]
#[test]
fn trail_route_uses_all_movement_exclusions() {
    let mut world = World::new();
    let exclusion = |center| positioning::SightExclusion {
        center,
        radius: 1.5,
        sight: crate::line_of_sight::ProjectedSight::Open,
    };
    let unit = world
        .spawn(positioning::CombatMovementExclusions(Some(vec![
            exclusion(Vec3::new(5.0, 0.0, 0.0)),
            exclusion(Vec3::new(5.0, 0.0, 3.0)),
        ])))
        .id();
    let origin = Vec3::ZERO;
    let unsafe_target = Vec3::new(10.0, 0.0, 3.0);
    let safe_target = Vec3::new(10.0, 0.0, -3.0);
    let unsafe_path = vec![Vec3::new(5.0, 0.0, 3.0), unsafe_target];
    let safe_path = vec![Vec3::new(0.0, 0.0, -3.0), safe_target];
    let selected = choose_route(
        &world,
        unit,
        origin,
        vec![safe_target, unsafe_target],
        vec![Ok(safe_path.clone()), Ok(unsafe_path.clone())],
    );
    assert_eq!(selected, Some((0, safe_target, safe_path)));
    assert_eq!(
        choose_route(
            &world,
            unit,
            origin,
            vec![unsafe_target],
            vec![Ok(unsafe_path)]
        ),
        None
    );
}

#[cfg(test)]
#[test]
#[ignore = "requires Ara-Kara navigation and ground assets"]
fn ara_kara_follower_retraces_safe_owner_trail_around_watcher() {
    use crate::followers::profile_behavior_tests::spawn_profile_fixture_with_plugins;
    use shared::components::{CombatStatus, UnitFactionTemplate, UnitLevel};
    use std::collections::HashMap;

    let (mut h, follower, _) = spawn_profile_fixture_with_plugins(209072, |_| {});
    h.app.world_mut().despawn(h.npc);
    let owner = h.caster;
    let map = InMap::continent(2660);
    let center = Vec3::new(-63.42, 133.01, -628.66);
    let start = Vec3::new(-78.70, 132.21, -621.23);
    let trail = [
        Vec3::new(-77.974045, 131.56999, -610.6789),
        Vec3::new(-81.32759, 131.51245, -626.7759),
        Vec3::new(-83.207375, 130.4844, -635.7988),
        Vec3::new(-84.65946, 132.70932, -643.39514),
        Vec3::new(-85.12416, 133.00012, -651.85925),
    ];
    let as_position = |p: Vec3| Position {
        x: p.x,
        y: p.y,
        z: p.z,
    };
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let maps = crate::maps::store::load_map_store(&db).unwrap();
    h.app.insert_resource(crate::ground::GroundMap::new(
        &crate::ground::ground_data_dir(),
        HashMap::from([(2660, maps.map(2660).unwrap().wdt_file_data_id)]),
    ));
    let nav = std::env::var_os("GAME_SERVER_NAV_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/ground/navmesh")
        });
    let mut native_map = ::navigation::NavigationMap::open(&nav.join("2660")).unwrap();
    native_map
        .complete_walk_path(start, trail[0])
        .expect("first trail point must be reachable on the real navmesh");
    crate::navigation::install_fixture(&mut h.app, Navigation::new(&nav));
    h.app
        .world_mut()
        .entity_mut(follower)
        .insert((map, as_position(start)));
    h.app
        .world_mut()
        .entity_mut(owner)
        .insert((map, CombatStatus(false)));
    let watcher = crate::spell_cast::test_harness::spawn_npc(&mut h.app, 0.0);
    h.app.world_mut().entity_mut(watcher).insert((
        map,
        as_position(center),
        UnitFactionTemplate(16),
        UnitLevel(80),
        crate::creature_aggro::AggroRange,
        crate::creature_ai::CreatureState::Idle,
    ));
    for (index, point) in trail.into_iter().enumerate() {
        if index == 2 {
            h.app
                .world_mut()
                .entity_mut(owner)
                .insert(CombatStatus(true));
        }
        h.app
            .world_mut()
            .entity_mut(owner)
            .insert(as_position(point));
        super::owner_trail::record(h.app.world_mut(), owner);
    }
    assert_eq!(
        super::owner_trail::points(h.app.world(), owner, map).last(),
        trail.last()
    );
    let exclusion = positioning::SightExclusion {
        center,
        radius: 17.0,
        sight: crate::line_of_sight::ProjectedSight::Open,
    };
    h.app
        .world_mut()
        .entity_mut(follower)
        .insert(positioning::CombatMovementExclusions(Some(vec![exclusion])));
    assert!(
        positioning::is_unpulled_hostile(h.app.world(), follower, watcher),
        "fixture watcher must be idle and hostile"
    );
    let exclusions = positioning::CombatMovementExclusions(Some(vec![exclusion]));
    assert!(
        trail
            .windows(2)
            .all(|leg| exclusions.permits(None, leg[0], leg[1])),
        "recorded owner trail must stay outside the watcher margin"
    );
    let mut previous = start;
    for _ in 0..600 {
        h.advance(1);
        let _ = advance(h.app.world_mut(), follower, owner, vec![watcher]);
        let at = h.app.world().get::<Position>(follower).copied().unwrap();
        let at = Vec3::new(at.x, at.y, at.z);
        assert!(
            exclusions.permits(None, previous, at),
            "unsafe step {previous:?} -> {at:?}"
        );
        previous = at;
        if at.distance(*trail.last().unwrap()) < 5.0 {
            break;
        }
    }
    assert!(
        previous.distance(*trail.last().unwrap()) < 5.0,
        "follower stopped at {previous:?} instead of reaching owner {:?}",
        trail.last().unwrap()
    );

    // A destination inside the idle watcher's sight has no safe route.
    clear(h.app.world_mut(), follower);
    h.app
        .world_mut()
        .entity_mut(owner)
        .insert(as_position(center));
    super::owner_trail::record(h.app.world_mut(), owner);
    let held_at = *h.app.world().get::<Position>(follower).unwrap();
    for _ in 0..140 {
        h.advance(1);
        assert!(advance(h.app.world_mut(), follower, owner, vec![watcher]));
        assert_eq!(*h.app.world().get::<Position>(follower).unwrap(), held_at);
    }
}

pub(super) fn clear(world: &mut World, unit: Entity) {
    let Some(state) = world.entity_mut(unit).take::<TrailFollow>() else {
        return;
    };
    let goal = Goal::OwnerTrail(state.owner.to_bits());
    if let Some(mut nav) = world.get_resource_mut::<Navigation>() {
        nav.cancel_query(unit, goal);
        if nav.route_destination(unit, goal).is_some() {
            nav.cancel_route(unit);
        }
    }
}
