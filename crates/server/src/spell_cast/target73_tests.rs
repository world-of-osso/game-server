//! Source73 destination planning, not the scripted actions of sampled native spells.
use super::test_harness::{Harness, harness, spawn_npc, world_db};
use super::*;
use shared::components::{PlayerMotion, Position, Rotation};

const RUTHERS: u32 = 120108;
const POISON: u32 = 65440;
const BLINK: u32 = 38981;
const BOON: u32 = 75080;
const SHACKLES: u32 = 346444;
const PULL: u32 = 260080;
const FORCE_SHATTER: u32 = 81037;
const SPLIT_SOUL: u32 = 224373;
const SAMPLES: usize = 24;
const YAW: f32 = 0.37;

fn scene() -> (Harness, Position) {
    let mut h = harness(
        Vec::new(),
        &[
            RUTHERS,
            POISON,
            BLINK,
            BOON,
            SHACKLES,
            PULL,
            FORCE_SHATTER,
            SPLIT_SOUL,
        ],
        1,
    )
    .expect("native target73 fixture");
    let origin = super::target148_tests::place_on_native_ground(&mut h);
    h.app.world_mut().entity_mut(h.caster).insert((
        origin,
        Rotation {
            x: 0.0,
            y: YAW + std::f32::consts::FRAC_PI_2,
            z: 0.0,
        },
    ));
    h.app.world_mut().entity_mut(h.npc).insert(Position {
        x: origin.x + 40.0,
        ..origin
    });
    (h, origin)
}

fn destination(h: &mut Harness, spell_id: u32) -> targets::Dest {
    let spell = world_db().unwrap().store.get(spell_id).unwrap().clone();
    plan_targets(h.app.world_mut(), h.caster, &spell, Some(h.npc), false)
        .expect("source73 planned destination")
        .into_iter()
        .find(|effect| effect.index == 0)
        .and_then(|effect| effect.dest)
        .expect("source73 emitted destination")
}

fn horizontal_distance(actual: targets::Dest, origin: Position) -> f32 {
    Vec2::new(actual.position.x - origin.x, actual.position.z - origin.z).length()
}

fn assert_floor(h: &mut Harness, actual: targets::Dest) {
    let map = *h.app.world().get::<crate::maps::InMap>(h.caster).unwrap();
    let probe = h
        .app
        .world_mut()
        .resource_mut::<crate::ground::GroundMap>()
        .creature_ground_in(map, actual.position);
    let crate::ground::GroundProbe::Supported(height) = probe else {
        panic!("native73 endpoint requires ground: {probe:?}");
    };
    assert!((actual.position.y - height).abs() < 0.001);
    assert!((actual.orientation - YAW).abs() < 0.001);
}

#[test]
fn toy_coverage_target73_centers_caster_not_selected_foreign_map_actor() {
    let (mut h, origin) = scene();
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(crate::maps::InMap::continent(0));
    for _ in 0..SAMPLES {
        let actual = destination(&mut h, RUTHERS);
        assert!(
            (horizontal_distance(actual, origin) - 5.0).abs() < 0.001,
            "{actual:?}"
        );
        assert_floor(&mut h, actual);
    }
}

#[test]
fn toy_coverage_target73_keeps_max_radius_not_random_min_max_disk() {
    let (mut h, origin) = scene();
    // Native65440 radius50 has Min10/Max15:73 keeps15 instead of sampling0..5.
    for _ in 0..SAMPLES {
        let actual = destination(&mut h, POISON);
        assert!(
            (horizontal_distance(actual, origin) - 15.0).abs() < 0.001,
            "{actual:?}"
        );
        assert_floor(&mut h, actual);
    }
}

#[test]
fn toy_coverage_target73_targetb_has_its_own_fixed_radius_after_random_targeta() {
    let (mut h, origin) = scene();
    // Native75080 [72,73], both radius37: B is fixed7 and caster-relative.
    for _ in 0..SAMPLES {
        let actual = destination(&mut h, BOON);
        assert!(
            (horizontal_distance(actual, origin) - 7.0).abs() < 0.001,
            "{actual:?}"
        );
    }
}

#[test]
fn toy_coverage_target73_missing_b_radius_inherits_native_targeta_entry() {
    let (mut h, origin) = scene();
    // Native38981 [1,73], A radius18/B absent: fixed15, not a radius0 origin.
    for _ in 0..SAMPLES {
        let actual = destination(&mut h, BLINK);
        assert!(
            (horizontal_distance(actual, origin) - 15.0).abs() < 0.001,
            "{actual:?}"
        );
    }
}

#[test]
fn toy_coverage_target73_no_radius_uses_source_caster_combat_reach_floor() {
    let (mut h, origin) = scene();
    h.app.world_mut().entity_mut(h.npc).insert(Position {
        x: origin.x + 3.0,
        ..origin
    });
    let reach = crate::scripted_ai::model_reach::combat_reach(h.app.world(), h.caster).unwrap();
    assert!(reach > 0.0);
    for _ in 0..SAMPLES {
        let actual = destination(&mut h, PULL);
        assert!(
            (horizontal_distance(actual, origin) - reach).abs() < 0.001,
            "{actual:?} reach{reach}"
        );
    }
}

#[test]
fn toy_coverage_target73_area_recipients_share_the_emitted_circle_sample() {
    let (mut h, origin) = scene();
    let map = *h.app.world().get::<crate::maps::InMap>(h.caster).unwrap();
    let points: Vec<_> = (-5..=5)
        .flat_map(|x| (-5..=5).map(move |z| (x, z)))
        .map(|(x, z)| {
            let entity = spawn_npc(&mut h.app, 0.0);
            let point = Position {
                x: origin.x + 2.0 * x as f32,
                z: origin.z + 2.0 * z as f32,
                ..origin
            };
            h.app.world_mut().entity_mut(entity).insert((point, map));
            (entity, Vec3::new(point.x, point.y, point.z))
        })
        .collect();
    let spell = world_db().unwrap().store.get(SHACKLES).unwrap().clone();
    for _ in 0..SAMPLES {
        let plan = plan_targets(h.app.world_mut(), h.caster, &spell, Some(h.npc), false)
            .expect("native346444 [73,16] plan");
        let effect = plan.iter().find(|effect| effect.index == 0).unwrap();
        let dest = effect.dest.expect("emitted73 center");
        assert!((horizontal_distance(dest, origin) - 5.0).abs() < 0.001);
        let expected: std::collections::HashSet<_> = points
            .iter()
            .filter(|(_, point)| point.distance(dest.position) <= 5.0)
            .map(|(entity, _)| *entity)
            .collect();
        assert!(
            !expected.is_empty(),
            "concrete field covers a second distinct recipient set"
        );
        let actual: std::collections::HashSet<_> = effect.units.iter().copied().collect();
        assert_eq!(
            actual, expected,
            "destination and recipients must share one sample"
        );
    }
}

#[test]
fn toy_coverage_target73_running_strafing_falling_bonus_excludes_walking_and_backward() {
    let (mut h, origin) = scene();
    let cases = [
        (PlayerMotion::FORWARD, 7.0),
        (PlayerMotion::STRAFE_LEFT, 7.0),
        (PlayerMotion::STRAFE_RIGHT, 7.0),
        (PlayerMotion::FALLING, 7.0),
        (PlayerMotion::FORWARD | PlayerMotion::WALKING, 5.0),
        (PlayerMotion::BACKWARD, 5.0),
        (0, 5.0),
    ];
    for (flags, expected) in cases {
        h.app
            .world_mut()
            .entity_mut(h.caster)
            .insert(PlayerMotion(flags));
        for _ in 0..SAMPLES {
            let actual = destination(&mut h, RUTHERS);
            assert!(
                (horizontal_distance(actual, origin) - expected).abs() < 0.001,
                "source motion{flags:x}: {actual:?} expected radius{expected}"
            );
        }
    }
}

#[test]
fn toy_coverage_target73_native_no_movement_radius_bonus_keeps_authored_extent() {
    let (mut h, origin) = scene();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(PlayerMotion(PlayerMotion::FORWARD));
    for _ in 0..SAMPLES {
        let actual = destination(&mut h, SPLIT_SOUL);
        assert!(
            (horizontal_distance(actual, origin) - 15.0).abs() < 0.001,
            "native224373 source NO_MOVEMENT_RADIUS_BONUS: {actual:?}"
        );
    }
}

#[test]
fn toy_coverage_target73_missing_collision_refuses_instead_of_unchecked_point() {
    let (mut h, _) = scene();
    h.app
        .world_mut()
        .remove_resource::<crate::line_of_sight::LineOfSight>();
    let spell = world_db().unwrap().store.get(RUTHERS).unwrap().clone();
    let result = plan_targets(h.app.world_mut(), h.caster, &spell, Some(h.npc), false);
    assert!(
        matches!(result, Err(CastFailure::Unsupported(ref reason)) if reason == "target73 requires collision data"),
        "{result:?}"
    );
}

#[test]
fn toy_coverage_target73_missing_ground_refuses_instead_of_unchecked_point() {
    let (mut h, _) = scene();
    h.app
        .world_mut()
        .remove_resource::<crate::ground::GroundMap>();
    let spell = world_db().unwrap().store.get(RUTHERS).unwrap().clone();
    let result = plan_targets(h.app.world_mut(), h.caster, &spell, Some(h.npc), false);
    assert!(
        matches!(result, Err(CastFailure::Unsupported(ref reason)) if reason == "target73 requires ground data"),
        "{result:?}"
    );
}

#[test]
fn toy_coverage_target73_force_location_retains_named_max_height_gap() {
    let (mut h, _) = scene();
    let spell = world_db()
        .unwrap()
        .store
        .get(FORCE_SHATTER)
        .unwrap()
        .clone();
    let result = plan_targets(h.app.world_mut(), h.caster, &spell, Some(h.npc), false);
    assert!(
        matches!(result, Err(CastFailure::Unsupported(ref reason)) if reason == "SPELL_DEST_FORCE_LOCATION_MAX_HEIGHT_ABSENT"),
        "{result:?}"
    );
}

#[test]
fn toy_coverage_target73_wikky_and_carp_remain_named_action_gaps() {
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let catalog = crate::toy_catalog::load(&conn).unwrap();
    for (item, reason) in [
        (86594, "source forage/presents and Pandaria interaction"),
        (131814, "source carp kick interaction"),
    ] {
        let unavailable = catalog.0[&item]
            .snapshot
            .unavailable_reason
            .as_deref()
            .unwrap_or_default();
        assert!(unavailable.contains(reason), "item{item}: {unavailable}");
    }
}
