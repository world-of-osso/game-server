//! Real source-driven FoF and Zenith children, without granting state.
use super::*;
use crate::spell_cast::effects::factor_capture::FactorCapture;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

type Run = super::super::Run;
const FOF: u32 = 113656;
const FOF_DAMAGE: u32 = 117418;
const JADEFIRE_DAMAGE: u32 = 1248815;
const ZENITH: u32 = 1249625;
const STOMP: u32 = 1272696;
const RSK: u32 = 107428;

fn prepare() -> Run {
    prepare_targets(1)
}

fn prepare_targets(targets: usize) -> Run {
    let mut run = super::super::prepare_measurement(269, targets).unwrap();
    let started = run.app.world().resource::<Time>().elapsed();
    super::super::install_fixture_lifetimes(&mut run, started);
    super::super::engage(&mut run);
    for _ in 0..3 {
        super::tests::cast(&mut run, 100780);
    }
    run
}

fn hits(run: &Run, spell: u32) -> usize {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(spell)
                && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_))
        })
        .count()
}

fn rsk_cooldown(run: &mut Run) -> f64 {
    let rotation = (run.spec_driver.take().unwrap() as Box<dyn std::any::Any>)
        .downcast::<Rotation>()
        .unwrap();
    let action = Action {
        name: "rising_sun_kick".into(),
        source_row: None,
        target: None,
        options: Default::default(),
    };
    let remaining = rotation
        .value_for(
            run.app.world_mut(),
            run.actor,
            &run.targets,
            &action,
            "cooldown.rising_sun_kick.full_recharge_time",
        )
        .unwrap();
    run.spec_driver = Some(rotation);
    remaining
}

#[test]
#[ignore = "actual FoF dispatches MID2 extra damage and selected Jadefire at channel end"]
fn literal_ww_native_source_children_fof_mid2_and_jadefire() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare();
    assert_eq!(
        run.app
            .world()
            .get::<crate::stats::EquippedItemModifiers>(run.actor)
            .unwrap()
            .temporary_enchants,
        vec![(shared::protocol::EquipmentSlot::MainHand, 8052)],
        "two-handed BIS applies only the equipped main-hand oil"
    );
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 1296621),
        "real equipped MID2 two-piece"
    );
    let before = hits(&run, FOF_DAMAGE);
    super::tests::cast(&mut run, FOF);
    for _ in 0..200 {
        if !crate::auras::has_aura(run.app.world(), run.actor, FOF) {
            break;
        }
        run.app.update();
    }
    assert!(
        hits(&run, JADEFIRE_DAMAGE) > 0,
        "selected Jadefire must actually damage after FoF"
    );
    assert_eq!(
        hits(&run, FOF_DAMAGE) - before,
        6,
        "native five ticks plus one actual MID2 child"
    );
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    assert!(
        !events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(FOF)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0),
        "SimC FoF root has no separate direct AP damage"
    );
}

#[test]
#[ignore = "actual Zenith starts native Stomp damage and resets the spent RSK cooldown"]
fn literal_ww_native_source_children_zenith_stomp_and_rsk_reset() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare();
    super::tests::cast(&mut run, RSK);
    assert!(
        rsk_cooldown(&mut run) > 1.0,
        "ordinary RSK really spends its cooldown"
    );
    super::tests::cast(&mut run, ZENITH);
    assert!(
        hits(&run, STOMP) > 0,
        "Zenith must actually dispatch its selected Stomp child"
    );
    assert!(
        rsk_cooldown(&mut run) <= 0.002,
        "Zenith resets actual RSK recovery"
    );
}

#[test]
#[ignore = "native FULL_MASK Stomp trigger ignores caster aura requirements"]
fn literal_ww_native_source_children_full_trigger_ignores_caster_aura() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare();
    assert!(!crate::auras::has_aura(run.app.world(), run.actor, 1291484));
    let actor = run.actor;
    let target = run.targets[0];
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, STOMP, Some(target), 1);
    assert!(
        hits(&run, STOMP) > 0,
        "FULL_MASK must ignore the native caster-aura prerequisite and deliver its child"
    );
}

#[test]
#[ignore = "real WDP child damage receives native Combo Strikes mastery"]
fn literal_ww_native_source_children_wdp_uses_native_mastery() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare();
    super::tests::cast(&mut run, RSK);
    super::tests::cast(&mut run, FOF);
    for _ in 0..200 {
        if !crate::auras::has_aura(run.app.world(), run.actor, FOF) {
            break;
        }
        run.app.update();
    }
    run.app.insert_resource(FactorCapture::new(
        run.actor,
        run.targets[0],
        451767,
        "wdp_mastery",
        5,
    ));
    super::tests::cast(&mut run, 152175);
    assert!(hits(&run, 451767) > 0, "actual WDP child delivery");
    let capture = run.app.world().resource::<FactorCapture>().export();
    let empowered = capture["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|row| row["applied_multipliers"].as_array())
        .flatten()
        .any(|factor| {
            factor["factor"] == "combo_strikes"
                && factor["value"].as_f64().is_some_and(|value| value > 1.0)
        });
    assert!(empowered, "actual WDP damage must include native mastery");
}

#[test]
#[ignore = "native RWK uses authored target scaling and divides its actual selected target count"]
fn literal_ww_native_source_children_rwk_uses_authored_split_multiplier() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_targets(3);
    run.app.insert_resource(FactorCapture::new(
        run.actor,
        run.targets[0],
        468179,
        "rwk_split",
        5,
    ));
    super::tests::cast(&mut run, 468179);
    assert_eq!(hits(&run, 468179), 3, "three actual native cone recipients");
    let capture = run.app.world().resource::<FactorCapture>().export();
    let world = run.app.world();
    let root = world
        .resource::<crate::spell_info::SpellStore>()
        .get(467307)
        .unwrap();
    let percent = root
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .base_points;
    let expected = (1.0 + 3.0 * f64::from(percent) / 100.0) / 3.0;
    let rows: Vec<_> = capture["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["event"] == "result")
        .collect();
    assert!(!rows.is_empty());
    for row in rows {
        let actual = row["factors"]["native_script_product"].as_f64().unwrap();
        assert!(
            (actual - expected).abs() < 0.00001,
            "RWK actual split multiplier={actual}, authored={expected}"
        );
    }
}
