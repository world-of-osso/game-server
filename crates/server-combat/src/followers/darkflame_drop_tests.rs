//! Approved2651 drop: actual native navigation, fall and follower movement.
use super::profile_behavior_tests::spawn_profile_fixture_with_plugins;
use crate::ground::{GroundMap, GroundProbe};
use crate::maps::InMap;
use crate::spell_cast::test_harness::Harness;
use bevy::prelude::*;
use shared::components::{Health, Position};
use std::collections::HashMap;
use std::path::PathBuf;

const MAP: u32 = 2651;
const WDT: u32 = 5_217_425;
const UPPER: Vec3 = Vec3::new(535.1, 86.534424, 110.0);
const DEPARTURE: Vec3 = Vec3::new(533.88, 86.534424, 110.0);
const LANDING: Vec3 = Vec3::new(533.88, 56.281814, 110.0);

fn assets() -> PathBuf {
    PathBuf::from(
        std::env::var_os("FOLLOWER_COMPONENT_ASSETS").expect("native collision asset root"),
    )
}

fn navigation_root() -> PathBuf {
    PathBuf::from(std::env::var_os("FOLLOWER_NAV_CANONICAL").expect("native navigation root"))
}

fn position(point: Vec3) -> Position {
    Position {
        x: point.x,
        y: point.y,
        z: point.z,
    }
}

fn point(position: Position) -> Vec3 {
    Vec3::new(position.x, position.y, position.z)
}

#[test]
#[ignore = "requires actual2651 navigation/ground assets"]
fn darkflame_drop_native_navigation_connects_downward_but_never_upward() {
    let mut map =
        ::navigation::NavigationMap::open(&navigation_root().join(MAP.to_string())).unwrap();
    let path = map
        .complete_path(UPPER, LANDING)
        .expect("approved one-way drop connects native components");
    assert!(path.first().unwrap().distance(UPPER) < 0.4);
    assert!(path.last().unwrap().distance(LANDING) < 0.4);
    assert!(
        map.complete_path(LANDING, UPPER).is_err(),
        "down-only link must not create an upward route"
    );
}

#[test]
#[ignore = "requires actual2527/2526/2661 navigation assets"]
fn darkflame_drop_policy_preserves_the_three_unapproved_map_gaps() {
    for (map_id, from, to) in [
        (
            2527,
            Vec3::new(-0.370431, 20.8162, -228.269),
            Vec3::new(-353.12274, -67.9066, 155.7234),
        ),
        (
            2526,
            Vec3::new(1410.22, 955.637, 2771.71),
            Vec3::new(1815.2512, 904.01917, 2783.9756),
        ),
        (
            2661,
            Vec3::new(2650.88, 101.042, 4835.88),
            Vec3::new(2649.0535, 110.600006, 5013.1484),
        ),
    ] {
        let mut map =
            ::navigation::NavigationMap::open(&navigation_root().join(map_id.to_string())).unwrap();
        assert!(
            map.complete_path(from, to).is_err(),
            "map{map_id} has no approved drop"
        );
    }
}

#[test]
#[ignore = "requires actual2651 navigation/ground assets"]
fn darkflame_drop_native_move_fall_lands_continuously_on_target_polygon() {
    use crate::scripted_ai::fall_movement;
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(GroundMap::new(&assets(), HashMap::from([(MAP, WDT)])));
    app.update();
    let unit = app
        .world_mut()
        .spawn((
            position(DEPARTURE),
            InMap::continent(MAP),
            Health {
                current: 100.0,
                max: 100.0,
            },
        ))
        .id();
    fall_movement::move_fall(app.world_mut(), unit, 0);
    assert!(
        app.world()
            .get::<fall_movement::FallMovement>(unit)
            .is_some(),
        "native ground must resolve the actual lower floor"
    );
    let mut collision = los::LosMap::new(&assets().join(format!("los/{MAP}")));
    let mut previous = DEPARTURE;
    let mut samples = 0;
    for _ in 0..100 {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(50));
        fall_movement::advance_falls(app.world_mut());
        let next = point(*app.world().get::<Position>(unit).unwrap());
        assert!((next.x - DEPARTURE.x).abs() < 0.001 && (next.z - DEPARTURE.z).abs() < 0.001);
        assert!(
            next.y <= previous.y && previous.y - next.y < 3.1,
            "continuous native gravity: {previous:?}->{next:?}"
        );
        assert!(
            collision.clear(previous + Vec3::Y, next + Vec3::Y).unwrap(),
            "fall crossed authored collision"
        );
        samples += usize::from(next.y < previous.y);
        previous = next;
    }
    assert!(
        samples > 20,
        "must fall over many native updates, not snap to landing"
    );
    assert!(
        (previous.y - LANDING.y).abs() < 0.05,
        "actual landing {previous:?}"
    );
    let mut ground = app.world_mut().resource_mut::<GroundMap>();
    assert!(
        matches!(ground.creature_ground_in(InMap::continent(MAP), previous), GroundProbe::Supported(y) if (y - LANDING.y).abs() < 0.05)
    );
    let mut map =
        ::navigation::NavigationMap::open(&navigation_root().join(MAP.to_string())).unwrap();
    assert!(
        map.complete_path(previous, LANDING)
            .unwrap()
            .last()
            .unwrap()
            .distance(LANDING)
            < 0.4
    );
    assert_eq!(
        app.world().get::<Health>(unit).unwrap().current,
        100.0,
        "FOLLOWER_NPC_FALL_DAMAGE_UNMODELED: do not add NPC damage"
    );
}

fn install_native_follow_movement(app: &mut App) {
    crate::creature_ai::install_follow_fixture(app);
    app.add_systems(
        Update,
        (
            crate::scripted_ai::movement::point_movement_system,
            crate::scripted_ai::fall_movement::advance_falls,
            crate::scripted_ai::fall_movement::release_finished_falls,
        )
            .chain(),
    );
}

fn create_native_follower_fixture() -> (Harness, Entity) {
    create_native_follower_fixture_for(209057)
}

fn create_native_follower_fixture_for(entry: u32) -> (Harness, Entity) {
    let (mut h, follower, _) =
        spawn_profile_fixture_with_plugins(entry, install_native_follow_movement);
    h.app
        .insert_resource(GroundMap::new(&assets(), HashMap::from([(MAP, WDT)])));
    crate::navigation::install_fixture(
        &mut h.app,
        crate::navigation::Navigation::new(&navigation_root()),
    );
    place_follower_on_upper(&mut h, follower, entry);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((position(UPPER), InMap::continent(MAP)));
    h.app
        .world_mut()
        .get_mut::<crate::networking::CombatTarget>(h.caster)
        .unwrap()
        .0 = None;
    (h, follower)
}

fn place_follower_on_upper(h: &mut Harness, follower: Entity, entry: u32) {
    let (speed_walk, speed_run) = {
        let template = &h
            .app
            .world()
            .resource::<crate::world_data::WorldData>()
            .templates[&entry];
        (template.speed_walk, template.speed_run)
    };
    h.app.world_mut().entity_mut(follower).insert((
        position(UPPER),
        InMap::continent(MAP),
        crate::creature_ai::SpawnPoint {
            x: UPPER.x,
            y: UPPER.y,
            z: UPPER.z,
            wander_distance: 0.0,
            speed_walk,
            speed_run,
        },
    ));
}

#[test]
#[ignore = "requires actual2651 navigation/ground and imported follower profile"]
fn darkflame_drop_follower_sim_traverses_only_after_upper_route_stage() {
    let (mut h, follower) = create_native_follower_fixture();
    h.advance(8);
    assert!(
        point(*h.app.world().get::<Position>(follower).unwrap()).distance(UPPER) < 0.1,
        "upper route stage must hold before leader proceeds"
    );
    // Actual lower-component boss placements: Blazikon order2000, Darkness order7000.
    let stages = [
        Vec3::new(406.7775, 56.68355, 155.27142),
        Vec3::new(188.47888, 59.8, 547.81714),
    ];
    let mut fall_samples = 0;
    for target in stages {
        *h.app.world_mut().get_mut::<Position>(h.caster).unwrap() = position(target);
        let mut arrived = false;
        for _ in 0..2400 {
            let before = point(*h.app.world().get::<Position>(follower).unwrap());
            h.advance(1);
            let after = point(*h.app.world().get::<Position>(follower).unwrap());
            assert!(
                after.distance(before) < 4.0,
                "follower teleported: {before:?}->{after:?}"
            );
            if before.y > 57.0 && before.y < 85.0 && after.y < before.y {
                fall_samples += 1;
            }
            if after.distance(target) < 15.0 {
                arrived = true;
                break;
            }
        }
        assert!(
            arrived,
            "follower failed authored lower stage {target:?}; feet {:?}",
            h.app.world().get::<Position>(follower)
        );
    }
    assert!(
        fall_samples > 15,
        "follower must traverse continuously rather than snap between floors"
    );
}

#[test]
#[ignore = "requires actual2651 navigation/ground and imported follower profile"]
fn darkflame_drop_idle_follower_descends_when_owner_is_directly_below() {
    let (mut h, follower) = create_native_follower_fixture();
    // Actual party-slot branch: slot2/4 places its settled follow point at UPPER.
    let upper_owner = Vec3::new(UPPER.x, UPPER.y, 114.0);
    let lower_owner = Vec3::new(upper_owner.x, LANDING.y, upper_owner.z);
    h.app
        .world_mut()
        .entity_mut(follower)
        .insert(crate::creature_ai::PartyFollowSlot::new(2, 4));
    h.app.world_mut().entity_mut(h.caster).insert((
        position(upper_owner),
        shared::components::Rotation {
            x: 0.0,
            y: -std::f32::consts::PI / 12.0,
            z: 0.0,
        },
    ));
    for owner in [upper_owner, lower_owner] {
        let probe = h
            .app
            .world_mut()
            .resource_mut::<GroundMap>()
            .ground_in(InMap::continent(MAP), owner);
        assert!(
            matches!(probe, GroundProbe::Supported(y) if (y-owner.y).abs()<0.05),
            "physical owner support required at {owner:?}: {probe:?}"
        );
    }
    let lower_follow_point = Vec3::new(UPPER.x, LANDING.y, UPPER.z);
    let mut native =
        ::navigation::NavigationMap::open(&navigation_root().join(MAP.to_string())).unwrap();
    let route = native
        .complete_path(UPPER, lower_follow_point)
        .expect("approved native route to the lower party follow point must exist");
    let owner_upper_surface = native
        .complete_path(upper_owner, upper_owner)
        .expect("upper owner must fit the unchanged native body profile");
    assert!(owner_upper_surface.last().unwrap().distance(upper_owner) < ::navigation::CELL_HEIGHT);
    h.advance(8);
    let upper = point(*h.app.world().get::<Position>(follower).unwrap());
    assert!(
        upper.distance(UPPER) < 0.1,
        "same-floor nearby owner correctly holds still"
    );
    *h.app.world_mut().get_mut::<Position>(h.caster).unwrap() = position(lower_owner);
    h.advance(80);
    let feet = point(*h.app.world().get::<Position>(follower).unwrap());
    eprintln!(
        "STACKED_FLOOR_FOLLOW_ADMISSION party_slot=2/4 route_corners={} owner={lower_owner:?} lower_follow_point={lower_follow_point:?} initial={upper:?} final={feet:?} goal_horizontal={} goal_vertical={} follow={:?}",
        route.len(),
        (feet.x - lower_follow_point.x).hypot(feet.z - lower_follow_point.z),
        feet.y - lower_follow_point.y,
        h.app
            .world()
            .get::<crate::creature_ai::FollowOwner>(follower)
    );
    // Change horizontal separation only; the same actor must then use the route.
    let far_follow_point = Vec3::new(406.7775, LANDING.y, 155.27142);
    native
        .complete_path(UPPER, far_follow_point)
        .expect("horizontal control party point is reachable");
    let far_owner = Vec3::new(
        far_follow_point.x + upper_owner.x - UPPER.x,
        lower_owner.y,
        far_follow_point.z + upper_owner.z - UPPER.z,
    );
    *h.app.world_mut().get_mut::<Position>(h.caster).unwrap() = position(far_owner);
    h.advance(80);
    let control = point(*h.app.world().get::<Position>(follower).unwrap());
    eprintln!("STACKED_FLOOR_FOLLOW_XZ_CONTROL owner={far_owner:?} feet={control:?}");
    assert!(
        control.y < UPPER.y - 20.0,
        "same follower/route must descend once XZ separation grows: {control:?}"
    );
    assert!(
        feet.y < LANDING.y + 1.0,
        "idle follower must not treat a reachable owner30yd below as already in range: {feet:?}"
    );
}

/// Live Follow failure 2026-10-29 in the Sprawling Trackyard: the slot point lies on a
/// walkable strip that the follower's polygon does not connect to.
const TRACKYARD_FROM: Vec3 = Vec3::new(297.12, 86.53, -38.58);
const TRACKYARD_TO: Vec3 = Vec3::new(331.23, 86.91, -36.79);

#[test]
#[ignore = "requires actual2651 navigation assets"]
fn darkflame_trackyard_follow_walks_the_incomplete_corridor_chase_stays_blocked() {
    let mut map =
        ::navigation::NavigationMap::open(&navigation_root().join(MAP.to_string())).unwrap();
    let mut plan = ::navigation::NavigationPlan::default();
    assert!(
        map.path_with_plan(TRACKYARD_FROM, TRACKYARD_TO, &mut plan)
            .is_err(),
        "non-Follow movers keep the disconnect as no path"
    );
    let path = map
        .follow_path(TRACKYARD_FROM, TRACKYARD_TO, &mut plan)
        .expect("Follow walks PATHFIND_INCOMPLETE");
    assert!(!path.complete);
    let end = *path.points.last().unwrap();
    let gap = |p: Vec3| (p.x - TRACKYARD_TO.x).hypot(p.z - TRACKYARD_TO.z);
    assert!(
        gap(end) < gap(TRACKYARD_FROM) - 10.0,
        "closest reachable polygon is toward the target: {end:?}"
    );
}

#[test]
#[ignore = "requires actual2651 navigation/ground and imported follower profile"]
fn darkflame_trackyard_follower_advances_toward_unreachable_slot_point() {
    let slot = crate::creature_ai::PartyFollowSlot::new(2, 4);
    // Facing PI - slot puts the slot point 4yd south of the owner: the live target.
    let owner = Vec3::new(TRACKYARD_TO.x, TRACKYARD_TO.y, TRACKYARD_TO.z + 4.0);
    let (mut h, follower) = create_native_follower_fixture();
    h.app.world_mut().entity_mut(follower).insert(slot);
    *h.app.world_mut().get_mut::<Position>(follower).unwrap() = position(TRACKYARD_FROM);
    h.app.world_mut().entity_mut(h.caster).insert((
        position(owner),
        shared::components::Rotation {
            x: 0.0,
            y: std::f32::consts::PI - slot.angle(),
            z: 0.0,
        },
    ));
    let gap = |p: Vec3| (p.x - TRACKYARD_TO.x).hypot(p.z - TRACKYARD_TO.z);
    let mut before = TRACKYARD_FROM;
    for _ in 0..80 {
        h.advance(1);
        let after = point(*h.app.world().get::<Position>(follower).unwrap());
        assert!(
            after.distance(before) < 4.0,
            "teleported: {before:?}->{after:?}"
        );
        before = after;
    }
    assert!(
        gap(before) < gap(TRACKYARD_FROM) - 10.0,
        "follower walks toward the slot point: {before:?}"
    );
}

/// Live 2026-10-29 instance 9: idle faction-0 Menial Laborers (210148) beside Ol' Waxbeard
/// never start an attack (`Creature::CanStartAttack` needs a hostile reaction), yet they
/// held the healer's regroup indefinitely. Only a watcher that would aggro holds a non-tank.
#[test]
#[ignore = "requires actual2651 navigation/ground and imported follower profile"]
fn darkflame_non_tank_approach_holds_only_for_watchers_that_would_aggro() {
    let from = Vec3::new(365.0142, 87.06531, -32.37358);
    let owner = Vec3::new(421.7417, 87.0782, -29.7178);
    let on_path = Vec3::new(393.4, 87.07, -31.0);
    // Actual factions: Royal Wicklighter 210812's 16 and Menial Laborer 210148's 0.
    for (faction, held) in [(16, true), (0, false)] {
        let (mut h, healer) = create_native_follower_fixture_for(209072);
        *h.app.world_mut().get_mut::<Position>(healer).unwrap() = position(from);
        *h.app.world_mut().get_mut::<Position>(h.caster).unwrap() = position(owner);
        let watcher = crate::spell_cast::test_harness::spawn_npc(&mut h.app, 0.0);
        h.app.world_mut().entity_mut(watcher).insert((
            position(on_path),
            InMap::continent(MAP),
            shared::components::UnitFactionTemplate(faction),
            shared::components::UnitLevel(80),
            crate::creature_aggro::AggroRange,
            crate::creature_ai::CreatureState::Idle,
        ));
        h.advance(1);
        let mut hold = None;
        for _ in 0..4 {
            hold = Some(super::positioning::hold_unsafe_approach(
                h.app.world_mut(),
                healer,
                h.caster,
            ));
            crate::navigation::drain_fixture(&mut h.app);
        }
        assert_eq!(hold, Some(held), "watcher faction {faction}");
    }
}

#[test]
#[ignore = "requires actual2651 navigation/ground and imported follower profile"]
fn candlelight_follower_walks_into_authored_box_and_receives_light() {
    let (mut h, follower) = create_native_follower_fixture();
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::maps::store::load_map_store(&conn).unwrap());
    let map = InMap::continent(MAP);
    crate::world_area_triggers::spawn_map(h.app.world_mut(), map, 205);
    let at = Position {
        x: 153.0,
        y: 74.2,
        z: 155.0,
    };
    h.app.world_mut().entity_mut(follower).insert((map, at));
    h.app.world_mut().entity_mut(h.npc).insert((
        map,
        shared::components::Npc {
            template_id: 208745,
            name: "The Candle King".into(),
        },
        shared::components::CombatStatus(true),
    ));
    let mut entered = false;
    for _ in 0..1000 {
        let before = *h.app.world().get::<Position>(follower).unwrap();
        let seeking = super::candlelight::seek(h.app.world_mut(), follower);
        h.advance(1);
        let after = *h.app.world().get::<Position>(follower).unwrap();
        assert!(
            point(after).distance(point(before)) < 2.0,
            "ordinary movement, not teleport"
        );
        if !seeking {
            entered = true;
            break;
        }
    }
    assert!(
        entered,
        "follower reached the authored light via native navigation"
    );
    crate::world_area_triggers::update_candlelight(h.app.world_mut());
    assert!(crate::auras::has_aura(h.app.world(), follower, 426125));
}
