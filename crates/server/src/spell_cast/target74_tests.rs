//! Source74 geometry only: these casts do not certify the sampled spells' actions.
use super::test_harness::{Harness, harness, spawn_npc, world_db};
use super::*;
use shared::components::{Position, Rotation};

const TARGET_YAW: f32 = 0.37;
const SAMPLES: usize = 24;
const CRAB: u32 = 289330;
const THWAP: u32 = 453588;
const BLOOD_AND_THUNDER: u32 = 128206;
const FORCE_JUMP: u32 = 140575;
const DEATHFORGE_MINE: u32 = 38107;
const THROW_DYNAMITE: u32 = 86088;

fn scene() -> (Harness, Position) {
    let mut h = harness(
        Vec::new(),
        &[
            CRAB,
            THWAP,
            BLOOD_AND_THUNDER,
            FORCE_JUMP,
            DEATHFORGE_MINE,
            THROW_DYNAMITE,
        ],
        1,
    )
    .expect("native source74 fixture");
    let origin = super::target148_tests::place_on_native_ground(&mut h);
    h.app.world_mut().entity_mut(h.npc).insert(Rotation {
        x: 0.0,
        y: TARGET_YAW,
        z: 0.0,
    });
    (h, origin)
}

fn destination(h: &mut Harness, spell_id: u32, index: usize) -> targets::Dest {
    let spell = world_db().unwrap().store.get(spell_id).unwrap().clone();
    plan_targets(h.app.world_mut(), h.caster, &spell, Some(h.npc), false)
        .expect("native74 cast plan")
        .into_iter()
        .find(|entry| entry.index == index)
        .and_then(|entry| entry.dest)
        .expect("native74 selected destination")
}

fn horizontal_distance(actual: targets::Dest, origin: Position) -> f32 {
    Vec2::new(actual.position.x - origin.x, actual.position.z - origin.z).length()
}

fn assert_native_floor(h: &mut Harness, actual: targets::Dest) {
    let map = *h.app.world().get::<crate::maps::InMap>(h.npc).unwrap();
    let floor = h
        .app
        .world_mut()
        .resource_mut::<crate::ground::GroundMap>()
        .creature_ground_in(map, actual.position);
    let crate::ground::GroundProbe::Supported(height) = floor else {
        panic!("actual target74 endpoint requires native ground: {floor:?}");
    };
    assert!(
        (actual.position.y - height).abs() < 0.001,
        "{actual:?} versus floor{height}"
    );
    assert!((actual.orientation - TARGET_YAW).abs() < 0.001);
}

fn spawn_enemy_field(h: &mut Harness, origin: Position) -> Vec<(Entity, Vec3)> {
    let map = *h.app.world().get::<crate::maps::InMap>(h.npc).unwrap();
    let original = (h.npc, Vec3::new(origin.x, origin.y, origin.z));
    (-7..=7)
        .flat_map(|x| (-7..=7).map(move |z| (x, z)))
        .map(|(x, z)| {
            let entity = spawn_npc(&mut h.app, 0.0);
            let position = Position {
                x: origin.x + x as f32,
                z: origin.z + z as f32,
                ..origin
            };
            h.app.world_mut().entity_mut(entity).insert((position, map));
            (entity, Vec3::new(position.x, position.y, position.z))
        })
        .chain(std::iter::once(original))
        .collect()
}

#[test]
fn toy_coverage_target74_area_recipients_use_the_same_sample_as_emitted_destination() {
    let (mut h, origin) = scene();
    let inputs = spawn_enemy_field(&mut h, origin);
    let spell = world_db()
        .unwrap()
        .store
        .get(THROW_DYNAMITE)
        .unwrap()
        .clone();
    // Native86088 [74,16]: source A radius5, B area radius2, no target cap.
    for _ in 0..SAMPLES {
        let plan = plan_targets(h.app.world_mut(), h.caster, &spell, Some(h.npc), false)
            .expect("native74-to-area cast plan");
        let effect = plan.iter().find(|effect| effect.index == 0).unwrap();
        let destination = effect.dest.expect("native74 emitted destination");
        assert!(horizontal_distance(destination, origin) <= 5.001);
        let expected: std::collections::HashSet<_> = inputs
            .iter()
            .filter(|(_, point)| point.distance(destination.position) <= 2.0)
            .map(|(entity, _)| *entity)
            .collect();
        let actual: std::collections::HashSet<_> = effect.units.iter().copied().collect();
        assert!(
            !expected.is_empty(),
            "concrete field must cover sampled destination"
        );
        assert_eq!(
            actual, expected,
            "recipients must use emitted center {destination:?}"
        );
    }
}

#[test]
fn toy_coverage_target74_crab_destination_uses_explicit_target_not_caster_or_missing_critter() {
    let (mut h, origin) = scene();
    // Native [150,74] has no current critter in this fixture.150 must not redirect74.
    let actual = destination(&mut h, CRAB, 0);
    assert!(horizontal_distance(actual, origin) <= 0.251, "{actual:?}");
    assert_native_floor(&mut h, actual);
}

#[test]
fn toy_coverage_target74_uses_radiusmax_without_caster_and_samples_max_minus_min() {
    let (mut h, origin) = scene();
    // Native453588 effect1 [25,74], B421:Radius3/Min3/Max6.
    // TC CalcRadius(nullptr, B) samples(6-3)*sqrt(rand), NOT(3-3) and NOT+Min.
    let distances: Vec<_> = (0..SAMPLES)
        .map(|_| {
            let actual = destination(&mut h, THWAP, 1);
            assert_native_floor(&mut h, actual);
            horizontal_distance(actual, origin)
        })
        .collect();
    assert!(
        distances.iter().all(|distance| *distance <= 3.001),
        "{distances:?}"
    );
    assert!(
        distances.iter().any(|distance| *distance > 0.5),
        "{distances:?}"
    );
    assert!(
        distances.iter().any(|distance| *distance < 2.5),
        "{distances:?}"
    );
}

#[test]
fn toy_coverage_target74_targeta_uses_its_own_radius_not_unused_targetb_radius() {
    let (mut h, origin) = scene();
    // Native128206 [74,0], A46:Min5/Max10; unused B77 is not selected.
    let distances: Vec<_> = (0..SAMPLES)
        .map(|_| {
            let actual = destination(&mut h, BLOOD_AND_THUNDER, 0);
            horizontal_distance(actual, origin)
        })
        .collect();
    assert!(
        distances.iter().all(|distance| *distance <= 5.001),
        "{distances:?}"
    );
    assert!(
        distances.iter().any(|distance| *distance < 4.0),
        "{distances:?}"
    );
}

#[test]
fn toy_coverage_target74_missing_b_radius_inherits_a_target_identity_and_fixed_radius() {
    let (mut h, origin) = scene();
    // Native38107 [6,74]/A15/B0: CalcRadius inherits A6, not random target74.
    // The direction is random but the radius is the fixed source3.
    for _ in 0..SAMPLES {
        let actual = destination(&mut h, DEATHFORGE_MINE, 0);
        assert!(
            (horizontal_distance(actual, origin) - 3.0).abs() < 0.001,
            "{actual:?}"
        );
    }
}

#[test]
fn toy_coverage_target74_missing_explicit_or_foreign_map_never_defaults_destination() {
    let (mut h, _) = scene();
    let spell = world_db().unwrap().store.get(CRAB).unwrap().clone();
    let effect = &spell.effects[0];
    let missing = targets::effect_destination(h.app.world_mut(), h.caster, None, &spell, effect);
    assert!(
        matches!(missing, Err(CastFailure::InvalidTarget)),
        "{missing:?}"
    );
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(crate::maps::InMap::continent(1));
    let foreign =
        targets::effect_destination(h.app.world_mut(), h.caster, Some(h.npc), &spell, effect);
    assert!(
        matches!(foreign, Err(CastFailure::InvalidTarget)),
        "{foreign:?}"
    );
}

#[test]
fn toy_coverage_target74_missing_collision_is_explicit_not_unchecked_geometry() {
    let (mut h, _) = scene();
    h.app
        .world_mut()
        .remove_resource::<crate::line_of_sight::LineOfSight>();
    let spell = world_db().unwrap().store.get(CRAB).unwrap().clone();
    let result = targets::effect_destination(
        h.app.world_mut(),
        h.caster,
        Some(h.npc),
        &spell,
        &spell.effects[0],
    );
    assert!(
        matches!(result, Err(CastFailure::Unsupported(ref reason)) if reason == "target74 requires collision data"),
        "{result:?}"
    );
}

#[test]
fn toy_coverage_target74_missing_ground_is_explicit_not_unchecked_geometry() {
    let (mut h, _) = scene();
    h.app
        .world_mut()
        .remove_resource::<crate::ground::GroundMap>();
    let spell = world_db().unwrap().store.get(CRAB).unwrap().clone();
    let result = targets::effect_destination(
        h.app.world_mut(),
        h.caster,
        Some(h.npc),
        &spell,
        &spell.effects[0],
    );
    assert!(
        matches!(result, Err(CastFailure::Unsupported(ref reason)) if reason == "target74 requires ground data"),
        "{result:?}"
    );
}

#[test]
fn toy_coverage_target74_force_location_retains_named_geometry_gap() {
    let (mut h, _) = scene();
    let spell = world_db().unwrap().store.get(FORCE_JUMP).unwrap().clone();
    let result = targets::effect_destination(
        h.app.world_mut(),
        h.caster,
        Some(h.npc),
        &spell,
        &spell.effects[0],
    );
    assert!(
        matches!(result, Err(CastFailure::Unsupported(ref reason)) if reason == "SPELL_DEST_FORCE_LOCATION_MAX_HEIGHT_ABSENT"),
        "{result:?}"
    );
}

#[test]
fn toy_coverage_target74_crab_ride_contract_stays_unavailable_without_source_action() {
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let catalog = crate::toy_catalog::load(&conn).unwrap();
    let reason = catalog.0[&166662].snapshot.unavailable_reason.as_deref();
    assert_eq!(
        reason,
        Some("unimplemented toy interaction: source battle-pet ride interaction")
    );
}
