//! SimC76f8118 sc_paladin.cpp1689–1710/1757–1783: actual HoW primary hits supply Sunrise.
use super::ret_dawnlight_consumer_tests::damage;
use super::ret_storm_gap_tests::remove_hero_selection;
use super::ret_tier_tests::{has, native_ret};
use super::*;
use crate::spell_cast::effects::factor_capture::FactorCapture;
use crate::spell_cast::test_harness::ObservedTriggeredCasts;

const HAMMER: u32 = 24275;
const SUNRISE: u32 = 431474;

fn native_effectiveness(run: &Run) -> f64 {
    let aura = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == SUNRISE && aura.caster == run.actor)
        .expect("selected native Second Sunrise");
    f64::from(
        aura.effects
            .iter()
            .find(|effect| effect.index == 1)
            .unwrap()
            .amount,
    ) / 100.0
}

fn observe_fight(run: &mut Run, effectiveness: f64) -> (usize, usize) {
    run.app.insert_resource(FactorCapture::new(
        run.actor,
        run.targets[0],
        HAMMER,
        "native_hammer_sunrise",
        2048,
    ));
    engage(run);
    advance_combat(run).expect("actual native300s follower combat");
    assert!(damage(run, HAMMER) > 0, "actual native Hammer must damage");
    let capture = run.app.world().resource::<FactorCapture>().export();
    let (roots, echoes) = count_hammer_damage(&capture, effectiveness);
    eprintln!("NATIVE_HAMMER_SUNRISE roots={roots} echoes={echoes} effectiveness={effectiveness}");
    (roots, echoes)
}

fn count_hammer_damage(capture: &Value, effectiveness: f64) -> (usize, usize) {
    let events = capture["events"].as_array().unwrap();
    let mut roots = 0;
    let mut echoes = 0;
    for event in events
        .iter()
        .filter(|event| event["event"] == "direct_amount")
    {
        let scale = event["applied_multipliers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["factor"] == "cast_effectiveness")
            .unwrap()["value"]
            .as_f64()
            .unwrap();
        let depth = event["native_depth"].as_u64().unwrap();
        roots += usize::from(depth == 0);
        echoes += usize::from(depth > 0 && (scale - effectiveness).abs() < 1e-6);
    }
    (roots, echoes)
}

#[test]
#[ignore = "actual300s selected ordinary HoW hits must deliver native-effectiveness Sunrise damage"]
fn ret_hammer_sunrise_selected_native_primary_hits_deliver_echo_damage() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(70, 1).unwrap();
    let effectiveness = native_effectiveness(&run);
    assert_eq!(effectiveness, 0.5, "native30 plus actual specialization20");
    let (roots, echoes) = observe_fight(&mut run, effectiveness);
    assert!(roots > 0, "ordinary native HoW hit prerequisite");
    assert!(
        echoes > 0,
        "selected native Second Sunrise produced no HoW echo damage"
    );
}

#[test]
#[ignore = "legal unspent Hero points retain ordinary HoW but cannot produce Sunrise damage"]
fn ret_hammer_sunrise_unselected_native_primary_hits_have_no_echo() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(70, 1).unwrap();
    let effectiveness = native_effectiveness(&run);
    remove_hero_selection(&mut run);
    assert!(!has(&run, SUNRISE));
    let (roots, echoes) = observe_fight(&mut run, effectiveness);
    assert!(roots > 0, "ordinary native HoW control must still hit");
    assert_eq!(echoes, 0);
}

#[test]
#[ignore = "same-ID native background HoW hits must not recursively produce Sunrise"]
fn ret_hammer_sunrise_native_background_hits_do_not_recurse() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret(4);
    assert!(has(&run, SUNRISE));
    run.app.init_resource::<ObservedTriggeredCasts>();
    for _ in 0..12 {
        crate::spell_cast::cast_triggered(
            run.app.world_mut(),
            run.actor,
            HAMMER,
            Some(run.targets[0]),
            1,
        );
        for _ in 0..40 {
            run.app.update();
        }
    }
    assert!(
        damage(&run, HAMMER) > 0,
        "native background control must hit"
    );
    let launches = run
        .app
        .world()
        .resource::<ObservedTriggeredCasts>()
        .0
        .iter()
        .filter(|(caster, spell, _)| *caster == run.actor && *spell == HAMMER)
        .count();
    assert_eq!(
        launches, 12,
        "only requested background casts, not recursive echoes"
    );
}
