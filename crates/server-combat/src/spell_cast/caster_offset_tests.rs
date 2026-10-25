//! Source41/43/48 caster destinations; sampled spell actions are not executed here.
use super::test_harness::{Harness, harness, world_db};
use super::*;
use shared::components::{ModelDisplay, Npc, PlayerMotion, Position, Rotation};
use shared::protocol::CombatLogKind;

const FRONT_RIGHT: u32 = 155043;
const BACK_LEFT: u32 = 105229;
const BACK: u32 = 279366;
const FRONT_RIGHT_NO_RADIUS: u32 = 10870;
const BACK_LEFT_NO_RADIUS: u32 = 8512;
const BACK_NO_RADIUS: u32 = 31985;
const BACK_LEFT_INHERITED: u32 = 35972;
const BACK_INHERITED: u32 = 29209;
const BACK_AREA: u32 = 126336;
const FORCE_FRONT_RIGHT: u32 = 97545;
const YAW: f32 = 0.37;

fn scene() -> (Harness, Position) {
    let mut h = harness(
        Vec::new(),
        &[
            FRONT_RIGHT,
            BACK_LEFT,
            BACK,
            FRONT_RIGHT_NO_RADIUS,
            BACK_LEFT_NO_RADIUS,
            BACK_NO_RADIUS,
            BACK_LEFT_INHERITED,
            BACK_INHERITED,
            BACK_AREA,
            FORCE_FRONT_RIGHT,
        ],
        1,
    )
    .unwrap();
    let origin = super::target148_tests::place_on_native_ground(&mut h);
    h.app.world_mut().entity_mut(h.caster).insert((
        origin,
        Rotation {
            x: 0.0,
            y: YAW + std::f32::consts::FRAC_PI_2,
            z: 0.0,
        },
    ));
    (h, origin)
}

fn plan(h: &mut Harness, id: u32) -> Result<Vec<PlannedEffect>, CastFailure> {
    let spell = world_db().unwrap().store.get(id).unwrap().clone();
    plan_targets(h.app.world_mut(), h.caster, &spell, Some(h.npc), false)
}

fn destination(h: &mut Harness, id: u32) -> targets::Dest {
    let spell = world_db().unwrap().store.get(id).unwrap().clone();
    //Pick the authored offset effect:279366's is index1, not its index0 self aura.
    //35972's effect43 action is unhandled; select targets without claiming that action.
    let effect = spell
        .effects
        .iter()
        .find(|effect| {
            effect
                .implicit_targets
                .iter()
                .any(|id| matches!(id, 41 | 43 | 48))
        })
        .expect("native source offset effect");
    targets::select_targets_and_destination(
        h.app.world_mut(),
        h.caster,
        Some(h.npc),
        &spell,
        effect,
        0,
        false,
    )
    .expect("source caster offset selected")
    .1
    .expect("source destination emitted")
}

fn assert_offset(h: &mut Harness, origin: Position, id: u32, angle: f32, distance: f32) {
    let actual = destination(h, id);
    let angle = YAW + angle;
    assert!(
        (actual.position.x - origin.x - distance * angle.cos()).abs() < 0.001,
        "{actual:?}"
    );
    assert!(
        (actual.position.z - origin.z + distance * angle.sin()).abs() < 0.001,
        "{actual:?}"
    );
    assert!((actual.orientation - YAW).abs() < 0.001);
    let map = *h.app.world().get::<crate::maps::InMap>(h.caster).unwrap();
    let probe = h
        .app
        .world_mut()
        .resource_mut::<crate::ground::GroundMap>()
        .creature_ground_in(map, actual.position);
    let crate::ground::GroundProbe::Supported(height) = probe else {
        panic!("native endpoint requires real ground: {probe:?}");
    };
    assert!((actual.position.y - height).abs() < 0.001);
}

#[test]
#[ignore = "caster-offset qualification; run explicitly"]
fn toy_coverage_caster_offset_native_directions_center_caster_not_foreign_selection() {
    let (mut h, origin) = scene();
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(crate::maps::InMap::continent(0));
    for (id, angle, distance) in [
        (FRONT_RIGHT, -std::f32::consts::FRAC_PI_4, 10.0),
        (BACK_LEFT, 3.0 * std::f32::consts::FRAC_PI_4, 2.0),
        (BACK, std::f32::consts::PI, 10.0),
    ] {
        assert_offset(&mut h, origin, id, angle, distance);
    }
}

#[test]
#[ignore = "caster-offset qualification; run explicitly"]
fn toy_coverage_caster_offset_missing_radius_defaults_corners_but_back_uses_reach() {
    let (mut h, origin) = scene();
    for (id, angle, distance) in [
        (FRONT_RIGHT_NO_RADIUS, -std::f32::consts::FRAC_PI_4, 3.0),
        (BACK_LEFT_NO_RADIUS, 3.0 * std::f32::consts::FRAC_PI_4, 3.0),
        (BACK_NO_RADIUS, std::f32::consts::PI, 1.5),
    ] {
        assert_offset(&mut h, origin, id, angle, distance);
    }
}

#[test]
#[ignore = "caster-offset qualification; run explicitly"]
fn toy_coverage_caster_offset_native_targetb_corner_default_and_back_inheritance_differ() {
    let (mut h, origin) = scene();
    //35972 B43 lacks its own entry: HasRadius(B) is false, so corner default3.
    assert_offset(
        &mut h,
        origin,
        BACK_LEFT_INHERITED,
        3.0 * std::f32::consts::FRAC_PI_4,
        3.0,
    );
    //29209 B48 lacks its own entry but is not a corner: CalcRadius inherits A20.
    assert_offset(&mut h, origin, BACK_INHERITED, std::f32::consts::PI, 20.0);
}

#[test]
#[ignore = "caster-offset qualification; run explicitly"]
fn toy_coverage_caster_offset_source_running_bonus_is_not_applied_while_walking() {
    let (mut h, origin) = scene();
    for (motion, distance) in [
        (PlayerMotion::FORWARD, 12.0),
        (PlayerMotion::FORWARD | PlayerMotion::WALKING, 10.0),
    ] {
        h.app
            .world_mut()
            .entity_mut(h.caster)
            .insert(PlayerMotion(motion));
        assert_offset(
            &mut h,
            origin,
            FRONT_RIGHT,
            -std::f32::consts::FRAC_PI_4,
            distance,
        );
    }
}

#[test]
#[ignore = "caster-offset qualification; run explicitly"]
fn toy_coverage_caster_offset_native_area_uses_final_emitted_center() {
    let (mut h, origin) = scene();
    let angle = YAW + std::f32::consts::PI;
    let center = Position {
        x: origin.x + 6.0 * angle.cos(),
        y: origin.y,
        z: origin.z - 6.0 * angle.sin(),
    };
    let entry = h.app.world().get::<Npc>(h.npc).unwrap().template_id;
    let display_id = crate::test_data::world_data().templates[&entry].models[0].display_id;
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert((center, ModelDisplay { display_id }));
    h.app
        .init_resource::<super::test_harness::ObservedCombatEvents>();
    let spell = world_db().unwrap().store.get(BACK_AREA).unwrap().clone();
    let paired = targets::select_targets_and_destination(
        h.app.world_mut(),
        h.caster,
        Some(h.npc),
        &spell,
        &spell.effects[0],
        0,
        false,
    )
    .unwrap();
    let selected_center = paired.1.expect("native48 selected destination");
    assert!(
        crate::line_of_sight::sees_area_center(h.app.world(), h.npc, selected_center.position),
        "authored Kobold display must supply the real LOS body"
    );
    assert_eq!(paired.0, vec![h.npc], "pre-defense source area membership");
    let effects = plan(&mut h, BACK_AREA).unwrap();
    let receipts = &h
        .app
        .world()
        .resource::<super::test_harness::ObservedCombatEvents>()
        .0;
    let native_miss = receipts.iter().any(|event| {
        let this_cast =
            event.spell_id == Some(BACK_AREA) && event.source == Some(h.caster.to_bits());
        let this_miss =
            event.target == Some(h.npc.to_bits()) && matches!(event.kind, CombatLogKind::Miss(_));
        this_cast && this_miss
    });
    for effect in effects {
        let dest = effect.dest.expect("native48/16 emitted destination");
        assert_eq!(
            dest.position, selected_center.position,
            "selected and emitted centers agree"
        );
        assert!((dest.position.x - center.x).abs() < 0.001);
        assert!((dest.position.z - center.z).abs() < 0.001);
        assert!(effect.units.iter().all(|unit| paired.0.contains(unit)));
        //Native126336 has magic defenses: disappearance must have a real Miss receipt.
        assert!(
            effect.units.contains(&h.npc) || native_miss,
            "unexplained recipient loss: {effect:?}; {receipts:?}"
        );
    }
}

#[test]
#[ignore = "caster-offset qualification; run explicitly"]
fn toy_coverage_caster_offset_missing_collision_and_ground_refuse_at_authority_boundary() {
    let (mut h, _) = scene();
    let collision = h
        .app
        .world_mut()
        .remove_resource::<crate::line_of_sight::LineOfSight>()
        .unwrap();
    assert!(
        matches!(plan(&mut h, BACK_LEFT), Err(CastFailure::Unsupported(reason)) if reason.contains("collision"))
    );
    h.app.insert_resource(collision);
    h.app
        .world_mut()
        .remove_resource::<crate::ground::GroundMap>();
    assert!(
        matches!(plan(&mut h, BACK_LEFT), Err(CastFailure::Unsupported(reason)) if reason.contains("ground"))
    );
}

#[test]
#[ignore = "caster-offset qualification; run explicitly"]
fn toy_coverage_caster_offset_native_force_height_retains_named_geometry_gap() {
    let (mut h, _) = scene();
    assert!(
        matches!(plan(&mut h, FORCE_FRONT_RIGHT), Err(CastFailure::Unsupported(reason)) if reason == "SPELL_DEST_FORCE_LOCATION_MAX_HEIGHT_ABSENT")
    );
}

#[test]
#[ignore = "caster-offset qualification; run explicitly"]
fn toy_coverage_caster_offset_arena_horn_cannot_admit_two_idle_neutral_gladiators() {
    let connection = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let catalog = crate::toy_catalog::load(&connection).unwrap();
    let reason = catalog.0[&108735]
        .snapshot
        .unavailable_reason
        .as_deref()
        .unwrap_or_default();
    assert!(
        reason.contains("source gladiator battle interaction"),
        "{reason}"
    );
}
