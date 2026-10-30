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

#[test]
#[ignore = "requires actual2526 navigation/ground and imported follower profile"]
fn academy_pad_follower_regroups_after_crawth_flight() {
    use crate::scripted_ai::jump_movement;
    let root = PathBuf::from(std::env::var_os("GAME_SERVER_GROUND_DIR").unwrap());
    let (mut h, follower, _) = spawn_profile_fixture_with_plugins(209057, |app| {
        install_native_follow_movement(app);
        app.add_plugins(crate::grid_activity::GridActivityPlugin);
        app.add_systems(
            Update,
            (
                jump_movement::release_finished_jumps,
                jump_movement::advance_jumps,
            )
                .chain(),
        );
    });
    h.app
        .insert_resource(GroundMap::new(&root, HashMap::from([(2526, 4_290_605)])));
    crate::navigation::install_fixture(
        &mut h.app,
        crate::navigation::Navigation::new(&root.join("navmesh")),
    );
    let departure = Vec3::new(2116.46, 928.646, 3105.17);
    let landing = Vec3::new(2306.25, 907.3357, 3052.91);
    let owner = Vec3::new(2331.7, 909.5034, 3050.197);
    let template = &h
        .app
        .world()
        .resource::<crate::world_data::WorldData>()
        .templates[&209057];
    let (speed_walk, speed_run) = (template.speed_walk, template.speed_run);
    h.app.world_mut().entity_mut(follower).insert((
        position(departure),
        InMap::continent(2526),
        crate::creature_ai::PartyFollowSlot::new(0, 4),
        crate::creature_ai::SpawnPoint {
            x: 1410.22,
            y: 955.637,
            z: 2771.71,
            wander_distance: 0.0,
            speed_walk,
            speed_run,
        },
    ));
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((position(departure), InMap::continent(2526)));
    h.app
        .world_mut()
        .get_mut::<crate::networking::CombatTarget>(h.caster)
        .unwrap()
        .0 = None;
    // Register the entrance spawn grid while the owner is still in its neighborhood.
    h.advance(2);
    jump_movement::move_jump(h.app.world_mut(), follower, 0, position(landing), 50.0, 0.0);
    *h.app.world_mut().get_mut::<Position>(h.caster).unwrap() = position(landing);
    h.advance(120);
    assert!(
        h.app
            .world()
            .get::<jump_movement::JumpMovement>(follower)
            .is_none()
    );
    *h.app.world_mut().get_mut::<Position>(h.caster).unwrap() = position(owner);
    h.advance(400);
    let feet = point(*h.app.world().get::<Position>(follower).unwrap());
    assert!(
        feet.distance(owner) < 18.0,
        "follower did not regroup: feet={feet:?}, owner={owner:?}"
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
fn registered_drop_moves_player_follower_and_pet_to_the_native_landing() {
    use bevy::ecs::system::RunSystemOnce;
    let (mut h, follower) = create_native_follower_fixture();
    let owner = h.caster;
    let pet = h
        .app
        .world_mut()
        .spawn((
            crate::hunter_pet::HunterPet {
                owner: follower,
                slot: 0,
            },
            InMap::continent(MAP),
            position(UPPER),
            Health {
                current: 100.0,
                max: 100.0,
            },
        ))
        .id();
    let edge = crate::test_support::graph::TransportEdge {
        from: 1,
        to: 2,
        source_guid: 0,
        radius: 1.0,
        departure: Some(DEPARTURE.to_array()),
        landing: LANDING.to_array(),
        action: crate::test_support::transport::Action::CollisionDrop,
    };
    crate::test_support::transport::activate(h.app.world_mut(), owner, None, &edge).unwrap();
    for _ in 0..150 {
        h.app
            .world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(50));
        crate::scripted_ai::movement::point_movement_system(h.app.world_mut());
        crate::scripted_ai::fall_movement::advance_falls(h.app.world_mut());
        h.app
            .world_mut()
            .run_system_once(crate::scripted_ai::fall_movement::release_finished_falls)
            .unwrap();
        crate::navigation::collision_drop::advance_collision_drops(h.app.world_mut());
    }
    for unit in [owner, follower, pet] {
        let feet = point(*h.app.world().get::<Position>(unit).unwrap());
        assert!(
            feet.distance(LANDING) <= ::navigation::CELL_HEIGHT,
            "unit {unit:?} landed at {feet:?}"
        );
        assert!(!crate::test_support::transport::in_progress(
            h.app.world(),
            unit
        ));
    }
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
#[ignore = "requires Windrunner navigation/ground and imported follower profile"]
fn followers_retrace_owner_curve_around_unpulled_watcher() {
    use shared::components::{Npc, UnitFactionTemplate, UnitLevel};
    use shared::group::GroupRole;
    let (mut h, first) = create_native_follower_fixture_for(209072);
    let map_id = 2805;
    h.app.insert_resource(GroundMap::new(
        &assets(),
        HashMap::from([(map_id, 6_099_383)]),
    ));
    let native = crate::spell_cast::test_harness::load_native_creature_world(&[6]);
    h.app
        .world_mut()
        .resource_mut::<crate::world_data::WorldData>()
        .templates
        .extend(native.templates);
    h.app
        .insert_resource(crate::line_of_sight::LineOfSight::open_arena());
    let owner = h.caster;
    crate::combat::combat_stop(h.app.world_mut(), owner);
    let profile = h
        .app
        .world()
        .get::<super::Follower>(first)
        .unwrap()
        .profile
        .clone();
    let faction = *h.app.world().get::<UnitFactionTemplate>(first).unwrap();
    let mut followers = vec![first];
    for slot in 1..4 {
        let actor = crate::spell_cast::test_harness::spawn_npc(&mut h.app, 0.0);
        let mut profile = profile.clone();
        if slot == 1 {
            profile.role = GroupRole::Tank;
        }
        h.app.world_mut().entity_mut(actor).insert((
            Npc {
                template_id: profile.entry,
                name: profile.name.clone(),
            },
            faction,
            UnitLevel(10),
            super::Follower { owner, profile },
            crate::creature_ai::PartyFollowSlot::new(slot, 4),
            crate::creature_ai::SpawnPoint {
                x: 5358.188,
                y: 101.434,
                z: 3146.1702,
                wander_distance: 0.0,
                speed_walk: 1.0,
                speed_run: 1.0,
            },
        ));
        followers.push(actor);
    }
    // Positions from the physical instance10 route through the Windrunner courtyard.
    let start = Vec3::new(5358.188, 101.434, 3146.1702);
    let end = Vec3::new(5331.647, 101.156, 3213.7322);
    // `content_creature` GUID63805106, not an invented unsupported position.
    let watcher_at = Vec3::new(5348.56, 101.28394, 3200.82);
    let watcher_floor = h
        .app
        .world_mut()
        .resource_mut::<GroundMap>()
        .ground_at(map_id, watcher_at);
    assert!(
        matches!(watcher_floor, GroundProbe::Supported(y) if (y - watcher_at.y).abs() < 1.0),
        "watcher must stand on native ground: {watcher_floor:?}"
    );
    h.app.world_mut().entity_mut(owner).insert((
        position(start),
        InMap::continent(map_id),
        UnitLevel(10),
    ));
    for &unit in &followers {
        h.app.world_mut().entity_mut(unit).insert((
            position(start),
            InMap::continent(map_id),
            crate::auras::AuraStatMods {
                movement_speed: 0.0,
                ..Default::default()
            },
        ));
    }
    let watcher = crate::spell_cast::test_harness::spawn_npc(&mut h.app, 0.0);
    h.app.world_mut().entity_mut(watcher).insert((
        position(watcher_at),
        InMap::continent(map_id),
        UnitFactionTemplate(16),
        UnitLevel(10),
        crate::creature_aggro::AggroRange,
        crate::creature_ai::CreatureState::Idle,
        shared::components::ThreatTable::default(),
    ));
    assert!(
        followers
            .iter()
            .all(|&unit| super::positioning::is_unpulled_hostile(h.app.world(), unit, watcher)),
        "fixture watcher must be hostile to every follower"
    );
    let mut geometry = bevy::ecs::system::SystemState::<crate::creature_aggro::AggroDistance>::new(
        h.app.world_mut(),
    );
    let radii: Vec<_> = followers
        .iter()
        .map(|&unit| {
            super::pull_route::read_native_radius(h.app.world(), &mut geometry, unit, watcher)
                .unwrap()
        })
        .collect();
    eprintln!("CURVE_TEST radii={radii:?} start={start:?} end={:?}", end);
    let mut collision = los::LosMap::new(&assets().join(format!("los/{map_id}")));
    let mut nav =
        ::navigation::NavigationMap::open(&navigation_root().join(map_id.to_string())).unwrap();
    let mut direct = vec![start];
    direct.extend(
        nav.complete_walk_path(start, end)
            .expect("direct corridor must exist"),
    );
    assert!(
        direct
            .windows(2)
            .any(
                |leg| super::pull_route::segment_distance_squared(watcher_at, leg[0], leg[1])
                    < radii[0] * radii[0]
            ),
        "direct follower corridor must cross the watcher radius: {direct:?}"
    );
    // A single waypoint can still force a path through the circle. Try two
    // grounded waypoints on its perimeter so the route can go around both sides.
    let mut waypoints = Vec::new();
    for radius in [18.0_f32, 24.0, 30.0] {
        for index in 0..12 {
            let angle = index as f32 * std::f32::consts::TAU / 12.0;
            let mut waypoint =
                watcher_at + Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius);
            if let GroundProbe::Supported(height) = h
                .app
                .world_mut()
                .resource_mut::<GroundMap>()
                .ground_at(map_id, waypoint)
            {
                if (height - watcher_at.y).abs() < 1.0 {
                    waypoint.y = height;
                    waypoints.push(waypoint);
                }
            }
        }
    }
    let safe_leg = |route: &[Vec3], collision: &mut los::LosMap| {
        route.windows(2).all(|leg| {
            super::pull_route::segment_distance_squared(watcher_at, leg[0], leg[1])
                >= (radii[0] + 3.0).powi(2)
                && collision.clear(leg[0] + Vec3::Y, leg[1] + Vec3::Y).unwrap()
        })
    };
    let mut owner_route = None;
    'search: for &first_waypoint in &waypoints {
        let Ok(first) = nav.complete_walk_path(start, first_waypoint) else {
            continue;
        };
        let mut first_leg = vec![start];
        first_leg.extend(first);
        if first_leg
            .last()
            .is_none_or(|p| p.distance(first_waypoint) >= 1.0)
            || !safe_leg(&first_leg, &mut collision)
        {
            continue;
        }
        for &second_waypoint in &waypoints {
            let Ok(middle) = nav.complete_walk_path(first_waypoint, second_waypoint) else {
                continue;
            };
            let Ok(last) = nav.complete_walk_path(second_waypoint, end) else {
                continue;
            };
            if middle
                .last()
                .is_none_or(|p| p.distance(second_waypoint) >= 1.0)
                || last.last().is_none_or(|p| p.distance(end) >= 1.0)
            {
                continue;
            }
            let mut route = first_leg.clone();
            route.extend(middle);
            route.extend(last);
            if safe_leg(&route, &mut collision) {
                owner_route = Some(route);
                break 'search;
            }
        }
    }
    let owner_route = owner_route.expect("a grounded curved owner route must avoid the watcher");
    eprintln!("CURVE_TEST safe owner route: {owner_route:?}");
    let mut owner_steps = 0;
    let mut observed_unsafe_corridor = false;
    for leg in owner_route.windows(2) {
        let length = leg[0].distance(leg[1]);
        let steps = (length / 0.35).ceil() as usize;
        for step in 1..=steps {
            let point = leg[0].lerp(leg[1], step as f32 / steps as f32);
            let previous = leg[0].lerp(leg[1], (step - 1) as f32 / steps as f32);
            let floor = h
                .app
                .world_mut()
                .resource_mut::<GroundMap>()
                .ground_at(map_id, point);
            assert!(
                matches!(floor, GroundProbe::Supported(y) if (y - point.y).abs() < 1.0),
                "owner path lacks ground at {point:?}: {floor:?}"
            );
            assert!(
                collision
                    .clear(previous + Vec3::Y, point + Vec3::Y)
                    .unwrap(),
                "owner path crosses collision at {point:?}"
            );
            h.app.world_mut().entity_mut(owner).insert(position(point));
            owner_steps += 1;
            if owner_steps == 80 {
                for &unit in &followers {
                    h.app
                        .world_mut()
                        .get_mut::<crate::auras::AuraStatMods>(unit)
                        .unwrap()
                        .movement_speed = 1.0;
                }
            }
            super::update_ai(h.app.world_mut());
            crate::navigation::drain_fixture(&mut h.app);
            h.advance(1);
            if !observed_unsafe_corridor && owner_steps % 10 == 0 {
                for (&unit, &radius) in followers.iter().zip(&radii) {
                    let feet = h.app.world().get::<Position>(unit).unwrap();
                    let from = Vec3::new(feet.x, feet.y, feet.z);
                    if let Ok(path) = nav.complete_walk_path(from, point) {
                        let mut route = vec![from];
                        route.extend(path);
                        observed_unsafe_corridor |= route.windows(2).any(|leg| {
                            super::pull_route::segment_distance_squared(watcher_at, leg[0], leg[1])
                                < radius * radius
                        });
                    }
                }
            }
            assert!(
                !crate::creature_aggro::read_player_proximity_acquisition(
                    h.app.world_mut(),
                    watcher,
                    owner
                )
                .unwrap(),
                "owner path must not pull watcher at {point:?}"
            );
            for &unit in &followers {
                assert!(
                    !crate::creature_sight::read_npc_proximity_acquisition(
                        h.app.world_mut(),
                        watcher,
                        unit
                    )
                    .unwrap(),
                    "watcher must not acquire a follower while owner walks"
                );
            }
        }
    }
    assert!(
        observed_unsafe_corridor,
        "a follower must encounter the unsafe direct corridor while the owner walks"
    );
    for _ in 0..800 {
        super::update_ai(h.app.world_mut());
        crate::navigation::drain_fixture(&mut h.app);
        h.advance(1);
        for (&unit, &radius) in followers.iter().zip(&radii) {
            let feet = point(*h.app.world().get::<Position>(unit).unwrap());
            let time = h.app.world().resource::<Time>().elapsed_secs_f64();
            assert!(
                !crate::creature_sight::read_npc_proximity_acquisition(
                    h.app.world_mut(),
                    watcher,
                    unit
                )
                .unwrap(),
                "watcher acquired follower {unit:?} at {feet:?}, radius={radius}, time={time}"
            );
            assert!(
                feet.distance(watcher_at) >= radius,
                "follower entered watcher radius at {feet:?}, radius {radius}"
            );
        }
    }
    for unit in followers {
        let feet = point(*h.app.world().get::<Position>(unit).unwrap());
        assert!(
            feet.distance(end) < 8.0,
            "follower did not arrive: {feet:?}"
        );
    }
    assert!(
        h.app
            .world()
            .get::<shared::components::ThreatTable>(watcher)
            .unwrap()
            .is_empty(),
        "unpulled watcher must remain idle"
    );
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
    let start = Vec3::new(153.0, 71.22188, 155.0);
    let GroundProbe::Supported(height) = h
        .app
        .world_mut()
        .resource_mut::<GroundMap>()
        .ground_at(MAP, start)
    else {
        panic!("actual Candle King approach floor");
    };
    let at = position(Vec3::new(start.x, height, start.z));
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
            "ordinary movement, not teleport: {before:?} -> {after:?}"
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

#[test]
#[ignore = "requires actual2651 navigation assets"]
fn darkflame_cart_post_gap_landing_walks_to_darkness() {
    let mut nav =
        ::navigation::NavigationMap::open(&navigation_root().join(MAP.to_string())).unwrap();
    // WCL WBYDm1N6JTZzCFnd fight3 actor2 t=6746889, projected onto native navmesh.
    let landing = Vec3::new(543.72, 87.4, 69.77);
    let darkness = Vec3::new(391.93, 87.26754, -27.99);
    let route = nav
        .complete_path(landing, darkness)
        .expect("post-gap landing connects to Darkness");
    assert!(route.first().unwrap().distance(landing) < 0.5);
    assert!(route.last().unwrap().distance(darkness) < 0.5);
}
