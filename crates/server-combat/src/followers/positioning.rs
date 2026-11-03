//! Follower approaches avoid idle hostile formations other than the target's own.
use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use shared::components::{Health, UnitFactionTemplate};

use super::pull_route;
use crate::creature_aggro::AggroDistance;
use crate::faction::{FactionTemplates, Reaction};
use crate::maps::InMap;
use crate::navigation::{Goal, Navigation};
use crate::spell_cast::targets;
use crate::system_states::with_state;
use ::navigation::worker::{Output, Query as NavQuery};

const HOLD_LOG_INTERVAL_SECONDS: f64 = 10.0;
/// Extra clearance for movement and sight updates between ticks, in yards.
const AGGRO_STEP_MARGIN: f32 = 0.5;
/// Spec follower-dungeons.md:41 (user-directed): an owner regroup without a safe route
/// resumes ordinary following after five seconds instead of holding indefinitely.
pub(super) const REGROUP_HOLD_SECONDS: f64 = 5.0;
/// Spacing of the sight samples along a path leg inside a watcher's radius, in yards.
const SIGHT_SAMPLE_SPACING: f32 = 0.5;

/// Live idle-hostile geometry used at the final combat-movement step.
#[derive(Clone, Copy)]
pub(crate) struct SightExclusion {
    pub center: Vec3,
    pub radius: f32,
    pub sight: crate::line_of_sight::ProjectedSight,
}

#[derive(Component)]
pub(crate) struct CombatMovementExclusions(pub Option<Vec<SightExclusion>>);

impl CombatMovementExclusions {
    pub(crate) fn permits(
        &self,
        los: Option<&crate::line_of_sight::LineOfSight>,
        from: Vec3,
        to: Vec3,
    ) -> bool {
        self.0.as_ref().is_some_and(|watchers| {
            watchers.iter().all(|watcher| {
                let start = from.distance_squared(watcher.center);
                let end = to.distance_squared(watcher.center);
                if start < watcher.radius * watcher.radius
                    && end >= start
                    && watcher.sight.sees(los, from)
                {
                    return true;
                }
                !leg_enters_sight((watcher.center, watcher.radius), from, to, |point| {
                    watcher.sight.sees(los, point)
                })
            })
        })
    }
}

pub(super) fn update_combat_exclusions(
    world: &mut World,
    unit: Entity,
    _follower: &super::Follower,
) {
    let Some(map) = world.get::<InMap>(unit).copied() else {
        world
            .entity_mut(unit)
            .insert(CombatMovementExclusions(None));
        return;
    };
    let planned = match world.get::<crate::creature_ai::PetMotion>(unit) {
        Some(crate::creature_ai::PetMotion::Chase(target)) => {
            super::pull_set::members(world, *target, unit)
        }
        _ => Default::default(),
    };
    let watchers: Vec<_> = world
        .resource::<crate::maps::units::MapUnits>()
        .in_map(map)
        .filter(|watcher| is_unpulled_hostile(world, unit, *watcher) && !planned.contains(watcher))
        .collect();
    let exclusions: Result<Vec<_>, String> =
        with_state::<AggroDistance, _>(world, |world, geometry| {
            watchers
                .into_iter()
                .map(|watcher| {
                    let center =
                        targets::position(world, watcher).ok_or("Idle hostile lacks position")?;
                    let radius = pull_route::read_native_radius(world, geometry, unit, watcher)?;
                    Ok(SightExclusion {
                        center,
                        radius: radius + AGGRO_STEP_MARGIN,
                        sight: crate::line_of_sight::ProjectedSight::capture(world, watcher, unit),
                    })
                })
                .collect()
        });
    if let Err(error) = &exclusions {
        warn!(?unit, %error, "Follower combat movement lacks native sight geometry");
    }
    world
        .entity_mut(unit)
        .insert(CombatMovementExclusions(exclusions.ok()));
}

#[derive(Component)]
struct HoldLog {
    target: Entity,
    last: f64,
    since: f64,
    reason: String,
}

fn note_hold(
    world: &mut World,
    unit: Entity,
    target: Entity,
    reason: &str,
    blockers: &[Entity],
) -> f64 {
    let now = world.resource::<Time>().elapsed_secs_f64();
    let previous = world
        .get::<HoldLog>(unit)
        .filter(|hold| hold.target == target);
    let since = previous.map_or(now, |hold| hold.since);
    if previous
        .is_none_or(|hold| hold.reason != reason || now - hold.last >= HOLD_LOG_INTERVAL_SECONDS)
    {
        let sim_ms = (now * 1000.0) as u64;
        let owner_exposure = world
            .get::<super::Follower>(unit)
            .filter(|follower| follower.owner == target)
            .and_then(|_| targets::position(world, target))
            .map(|point| blockers_on_path(world, target, &[point, point], blockers.to_vec()));
        warn!(
            ?unit,
            ?target,
            sim_ms,
            reason,
            ?blockers,
            ?owner_exposure,
            "Follower approach held"
        );
        world.entity_mut(unit).insert(HoldLog {
            target,
            last: now,
            since,
            reason: reason.to_owned(),
        });
    }
    since
}

fn hold_unresolved_approach(world: &mut World, unit: Entity, target: Entity, reason: &str) -> bool {
    let since = note_hold(world, unit, target, reason, &[]);
    let now = world.resource::<Time>().elapsed_secs_f64();
    let is_owner = world
        .get::<super::Follower>(unit)
        .is_some_and(|follower| follower.owner == target);
    if is_owner && now - since >= REGROUP_HOLD_SECONDS {
        return false;
    }
    hold_position(world, unit);
    true
}

fn hold_position(world: &mut World, unit: Entity) {
    world
        .entity_mut(unit)
        .remove::<crate::creature_ai::FollowOwner>()
        .insert(crate::creature_ai::CreatureState::Idle);
}

pub(super) fn hold_unsafe_approach(world: &mut World, unit: Entity, target: Entity) -> bool {
    let is_owner = world
        .get::<super::Follower>(unit)
        .is_some_and(|follower| follower.owner == target);
    if !is_owner {
        super::trail_follow::clear(world, unit);
    }
    if is_owner && super::trail_follow::active(world, unit, target) {
        return super::trail_follow::advance(world, unit, target, Vec::new());
    }
    let blockers = match approach_enters_unpulled_sight(world, unit, target) {
        Ok(None) => {
            return hold_unresolved_approach(world, unit, target, "waiting for navigation");
        }
        Ok(Some(blockers)) => blockers,
        Err(error) => {
            return hold_unresolved_approach(world, unit, target, &error);
        }
    };
    if blockers.is_empty() {
        super::trail_follow::clear(world, unit);
        world.entity_mut(unit).remove::<HoldLog>();
        return false;
    }
    note_hold(world, unit, target, "unpulled hostile sight", &blockers);
    hold_position(world, unit);
    if is_owner {
        return super::trail_follow::advance(world, unit, target, blockers);
    }
    true
}

pub(super) fn approach_enters_unpulled_sight(
    world: &mut World,
    unit: Entity,
    target: Entity,
) -> Result<Option<Vec<Entity>>, String> {
    let map = *world
        .get::<InMap>(unit)
        .ok_or("Follower positioning lacks map")?;
    let from = targets::position(world, unit).ok_or("Follower positioning lacks origin")?;
    let to =
        targets::position(world, target).ok_or("Follower positioning lacks target position")?;
    let planned = if targets::relation_matches(world, unit, target, targets::Relation::Enemy) {
        super::pull_set::members(world, target, unit)
    } else {
        Default::default()
    };
    let watchers: Vec<_> = world
        .resource::<crate::maps::units::MapUnits>()
        .in_map(map)
        .filter(|watcher| is_unpulled_hostile(world, unit, *watcher) && !planned.contains(watcher))
        .collect();
    if watchers.is_empty() {
        return Ok(Some(Vec::new()));
    }
    let Some(mut path) = read_approach_path(world, unit, target, map, from, to)? else {
        return Ok(None);
    };
    path.insert(0, from);
    blockers_on_path(world, unit, &path, watchers).map(Some)
}

pub(super) fn route_enters_unpulled_sight(
    world: &mut World,
    unit: Entity,
    target: Entity,
    path: &[Vec3],
) -> Result<bool, String> {
    let map = *world
        .get::<InMap>(unit)
        .ok_or("Follower positioning lacks map")?;
    let planned = if targets::relation_matches(world, unit, target, targets::Relation::Enemy) {
        super::pull_set::members(world, target, unit)
    } else {
        Default::default()
    };
    let watchers = world
        .resource::<crate::maps::units::MapUnits>()
        .in_map(map)
        .filter(|watcher| is_unpulled_hostile(world, unit, *watcher) && !planned.contains(watcher))
        .collect();
    Ok(!blockers_on_path(world, unit, path, watchers)?.is_empty())
}

fn blockers_on_path(
    world: &mut World,
    unit: Entity,
    path: &[Vec3],
    watchers: Vec<Entity>,
) -> Result<Vec<Entity>, String> {
    with_state::<AggroDistance, _>(world, |world, geometry| {
        blockers_with(world, geometry, unit, path, watchers)
    })
}

fn blockers_with(
    world: &World,
    geometry: &mut SystemState<AggroDistance>,
    unit: Entity,
    path: &[Vec3],
    watchers: Vec<Entity>,
) -> Result<Vec<Entity>, String> {
    let mut blockers = Vec::new();
    for watcher in watchers {
        let radius = pull_route::read_native_radius(world, geometry, unit, watcher)?;
        let center = targets::position(world, watcher).ok_or("Unpulled watcher lacks position")?;
        let sight = crate::line_of_sight::ProjectedSight::capture(world, watcher, unit);
        let los = world.get_resource::<crate::line_of_sight::LineOfSight>();
        if path.windows(2).any(|leg| {
            leg_enters_sight(
                (center, radius + AGGRO_STEP_MARGIN),
                leg[0],
                leg[1],
                |point| sight.sees(los, point),
            )
        }) {
            blockers.push(watcher);
        }
    }
    Ok(blockers)
}

/// The one idle-hostile exclusion predicate for a path leg. TC ea5113cce
/// Creature.cpp:2107 `Creature::CanStartAttack` ends `return IsWithinLOSInMap(who);`
/// (:2143): a leg enters a watcher's sight only where it is inside the radius and the
/// watcher sees the unit standing there.
pub(super) fn leg_enters_sight(
    (center, radius): (Vec3, f32),
    from: Vec3,
    to: Vec3,
    sees: impl Fn(Vec3) -> bool,
) -> bool {
    if pull_route::segment_distance_squared(center, from, to) >= radius * radius {
        return false;
    }
    let leg = to - from;
    let closest = from
        + leg * ((center - from).dot(leg) / leg.length_squared().max(f32::EPSILON)).clamp(0.0, 1.0);
    let steps = (leg.length() / SIGHT_SAMPLE_SPACING).ceil().max(1.0) as usize;
    (0..=steps)
        .map(|step| from.lerp(to, step as f32 / steps as f32))
        .chain([closest])
        .filter(|point| point.distance_squared(center) < radius * radius)
        .any(sees)
}

pub(super) fn in_target_formation(world: &World, target: Entity, watcher: Entity) -> bool {
    if target == watcher {
        return true;
    }
    let (Some(target), Some(watcher)) = (
        world.get::<crate::zones::CreatureOrigin>(target),
        world.get::<crate::zones::CreatureOrigin>(watcher),
    ) else {
        return false;
    };
    if target.guid == 0 || watcher.guid == 0 {
        return false;
    }
    let Some(data) = world.get_resource::<crate::world_data::WorldData>() else {
        return false;
    };
    let pack = |guid| {
        super::pack_identity::pack_guid(
            guid,
            data.formations.get(&guid).map(|entry| entry.leader_guid),
        )
    };
    pack(target.guid) == pack(watcher.guid)
}

pub(super) fn approach_goal(world: &World, unit: Entity, target: Entity) -> Goal {
    if world
        .get::<super::Follower>(unit)
        .is_some_and(|follower| follower.owner == target)
    {
        Goal::RegroupApproach(target.to_bits())
    } else {
        Goal::Approach(target.to_bits())
    }
}

fn read_approach_path(
    world: &mut World,
    unit: Entity,
    target: Entity,
    map: InMap,
    from: Vec3,
    to: Vec3,
) -> Result<Option<Vec<Vec3>>, String> {
    #[cfg(any(test, feature = "test-support"))]
    if let Some(space) = world.get_resource::<crate::test_support::Space>() {
        return space.route(from, to).map(Some);
    }
    let goal = approach_goal(world, unit, target);
    let Some(result) = world
        .get_resource_mut::<Navigation>()
        .ok_or("Follower positioning requires authored navigation")?
        .query_deferred(unit, map.map_id, goal, NavQuery::CompleteRoute { from, to })
    else {
        return Ok(None);
    };
    match result? {
        Some(Output::CompleteRoute(route)) => {
            if !route.complete {
                return Err(route
                    .reason
                    .unwrap_or_else(|| "Follower positioning route is incomplete".into()));
            }
            if route.points.is_empty() {
                return Err("Follower positioning route is empty".into());
            }
            Ok(Some(route.points))
        }
        None => Err("Follower positioning has no authored navigation bake".into()),
        Some(_) => Err("Follower positioning received a non-path result".into()),
    }
}

#[cfg(test)]
#[test]
fn halls_follower_approach_resolves_all_corridor_segments() {
    let root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/ground/navmesh");
    let mut world = World::new();
    world.insert_resource(Navigation::new(&root));
    let unit = world.spawn_empty().id();
    let owner = world.spawn_empty().id();
    let map = InMap::continent(2527);
    let from = Vec3::new(3.4296, -130.648, -227.02);
    let to = Vec3::new(-0.37, 20.816, -228.269);
    assert!(
        read_approach_path(&mut world, unit, owner, map, from, to)
            .unwrap()
            .is_none()
    );
    world.resource_mut::<Navigation>().drain_until_idle();
    let points = read_approach_path(&mut world, unit, owner, map, from, to)
        .unwrap()
        .expect("complete route");
    assert!(
        points.len() > 256,
        "route must include all segments: {}",
        points.len()
    );
    assert!(points.last().unwrap().distance(to) < 1.0);
    assert!(points.iter().any(|point| point.y < -100.0));
    assert!(points.iter().any(|point| point.y > 10.0));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::followers::profile_behavior_tests::spawn_profile_fixture_with_plugins;
    use crate::test_support::graph::{Graph, Pack};
    use shared::components::{
        CharacterAppearance, CharacterVisual, ModelDisplay, Position, UnitLevel,
    };

    #[test]
    fn held_approach_logs_blockers_and_throttles_repeats() {
        use std::io::Write;
        use std::sync::{Arc, Mutex};

        struct Capture(Arc<Mutex<Vec<u8>>>);
        impl Write for Capture {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().extend_from_slice(bytes);
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let output = Arc::new(Mutex::new(Vec::new()));
        let captured = output.clone();
        let subscriber = bevy::log::tracing_subscriber::fmt()
            .with_ansi(false)
            .with_writer(move || Capture(captured.clone()))
            .finish();
        bevy::log::tracing::subscriber::with_default(subscriber, || {
            let mut world = World::new();
            world.insert_resource(Time::<()>::default());
            let unit = world.spawn_empty().id();
            let target = world.spawn_empty().id();
            let blocker = world.spawn_empty().id();
            note_hold(
                &mut world,
                unit,
                target,
                "unpulled hostile sight",
                &[blocker],
            );
            note_hold(
                &mut world,
                unit,
                target,
                "unpulled hostile sight",
                &[blocker],
            );
            world
                .resource_mut::<Time>()
                .advance_by(std::time::Duration::from_secs(11));
            note_hold(
                &mut world,
                unit,
                target,
                "unpulled hostile sight",
                &[blocker],
            );
        });
        let text = String::from_utf8(output.lock().unwrap().clone()).unwrap();
        assert_eq!(text.matches("Follower approach held").count(), 2, "{text}");
        assert!(text.contains("unpulled hostile sight"), "{text}");
        assert!(text.contains("blockers=["), "{text}");
    }

    #[test]
    fn character_visual_follower_keeps_following_when_watchers_are_off_route() {
        let (mut h, follower, _) = spawn_profile_fixture_with_plugins(209072, |_| {});
        let db = rusqlite::Connection::open_with_flags(
            crate::test_data::world_db_path(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        h.app
            .insert_resource(crate::test_data::world_data().clone());
        h.app
            .insert_resource(crate::scripted_ai::model_reach::load_model_reaches(&db).unwrap());
        h.app.insert_resource(crate::test_support::Space(Graph {
            map_id: 0,
            entrance: [0.0, 0.0],
            entrance_height: 0.0,
            entrance_pack: 1,
            packs: vec![Pack {
                id: 1,
                position: [0.0, 0.0],
                height: 0.0,
                spawns: vec![],
            }],
            edges: vec![],
            teleports: vec![],
            transports: vec![],
            flights: vec![],
            unreachable: vec![],
            route: vec![],
            follower_difficulty: 0,
            follower_boss_entries: vec![],
            follower_boss_sources: default(),
            follower_boss_bits: default(),
            follower_encounter_actor_bits: default(),
        }));
        h.app
            .world_mut()
            .entity_mut(follower)
            .remove::<ModelDisplay>()
            .insert((
                CharacterVisual {
                    race: 1,
                    class: 5,
                    appearance: CharacterAppearance::default(),
                },
                UnitLevel(90),
                Position {
                    x: 10.0,
                    y: 0.0,
                    z: 0.0,
                },
            ));
        h.app.world_mut().entity_mut(h.npc).insert((
            UnitLevel(90),
            Position {
                x: 100.0,
                y: 0.0,
                z: 100.0,
            },
            crate::creature_aggro::AggroRange,
            crate::creature_ai::CreatureState::Idle,
        ));
        let owner = h.caster;
        assert!(!hold_unsafe_approach(h.app.world_mut(), follower, owner));
        assert_eq!(
            h.app
                .world()
                .get::<crate::creature_ai::FollowOwner>(follower)
                .unwrap()
                .owner,
            owner
        );
    }
}

pub(super) fn is_unpulled_hostile(world: &World, unit: Entity, watcher: Entity) -> bool {
    // Match CreatureSight's optional react state: absent means Aggressive.
    let aggressive = world
        .get::<crate::creature_aggro::AggroRange>(watcher)
        .is_some()
        && world
            .get::<crate::scripted_ai::ReactState>(watcher)
            .is_none_or(|react| *react == crate::scripted_ai::ReactState::Aggressive);
    let alive = world
        .get::<Health>(watcher)
        .is_some_and(|health| health.current > 0.0);
    let idle = world
        .get::<crate::creature_ai::CreatureState>(watcher)
        .is_some_and(|state| state.is_idle());
    // Creature::CanStartAttack: aggro needs a hostile reaction, not just attackability.
    let hostile = world
        .get::<UnitFactionTemplate>(watcher)
        .is_some_and(|faction| {
            let follower = world.get::<UnitFactionTemplate>(unit).map_or(0, |f| f.0);
            world
                .resource::<FactionTemplates>()
                .reaction(faction.0, follower)
                == Reaction::Hostile
        });
    aggressive
        && alive
        && idle
        && hostile
        && targets::relation_matches(world, unit, watcher, targets::Relation::Enemy)
}
