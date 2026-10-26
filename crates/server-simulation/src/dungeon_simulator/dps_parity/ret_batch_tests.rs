//! Native resource and cast-volume boundaries from the pinned Ret reference.
use super::ret_dawnlight_consumer_tests::{damage, owned_dot_at, wait_for_damage};
use super::ret_dawnlight_tests::{budget, cast_and_damage};
use super::ret_purpose_tests::{current_holy_power, earn_finisher_power};
use super::ret_tier_tests::{cast_finisher, has, native_ret};
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "native Dawnlight reapplication adds fresh duration to remaining duration"]
fn dps_parity_ret_batch_dawnlight_two_real_spenders_extend() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret(4);
    cast_and_damage(&mut run, 255937);
    cast_finisher(&mut run, 383328);
    wait_for_damage(&mut run, 383328);
    assert_eq!(budget(&run).unwrap().stacks, 2);
    let fresh = f64::from(owned_dot_at(&run, run.targets[0]).unwrap().duration_ms) / 1000.0;
    // This duration test needs ordinary Crusading Strikes to fund its next
    // spender before the first DoT expires. The Wake-only helper does not
    // start this independent resource producer (sc_paladin.cpp888–916).
    super::literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        super::literal_rotation::execution::Execution::AutoAttack(run.targets[0]),
    )
    .unwrap();
    earn_finisher_power(&mut run, 383328);
    let old = owned_dot_at(&run, run.targets[0])
        .unwrap()
        .expires_at
        .unwrap();
    assert!(old > crate::auras::aura_now(run.app.world()));
    let before = damage(&run, 383328);
    cast_finisher(&mut run, 383328);
    for _ in 0..200 {
        if budget(&run).is_some_and(|aura| aura.stacks == 1) {
            break;
        }
        run.app.update();
    }
    assert!(damage(&run, 383328) > before);
    assert_eq!(budget(&run).unwrap().stacks, 1);
    let expiry = owned_dot_at(&run, run.targets[0])
        .unwrap()
        .expires_at
        .unwrap();
    // SimC hpp1492–1496 extends300ms before the spender reapplies Dawnlight.
    let impact_extension = 0.3;
    assert!(
        (expiry - old - fresh - impact_extension).abs() < 0.001,
        "old={old}, fresh={fresh}, impact={impact_extension}, actual={expiry}"
    );
}

#[test]
#[ignore = "ordinary Wrath with selected Hammer talent must produce its native mode"]
fn dps_parity_ret_batch_wrath_produces_hammer_mode() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret(4);
    cast_finisher(&mut run, 31884);
    assert!(has(&run, 31884));
    assert!(
        has(&run, 1277026),
        "selected Hammer mode must follow actual Wrath"
    );
}

#[test]
#[ignore = "native Judgment and Hammer splash must not hit their primary twice"]
fn dps_parity_ret_batch_primary_is_not_its_own_splash() {
    for spell in [20271, 24275] {
        let _seed = crate::random::seed_scope(70448);
        let mut run = native_ret(4);
        if spell == 24275 {
            cast_finisher(&mut run, 31884);
        }
        cast_finisher(&mut run, spell);
        wait_for_damage(&mut run, spell);
        let hits = run
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.target == Some(run.targets[0].to_bits())
                    && event.spell_id == Some(spell)
                    && event.kind == CombatLogKind::Damage
            })
            .count();
        assert_eq!(
            hits, 1,
            "one ordinary native{spell} must not damage primary twice"
        );
    }
}

#[test]
#[ignore = "Walk into Light mirrors actual Hammer during real Wrath into native Blade"]
fn dps_parity_ret_batch_walk_into_light_mirrors_hammer() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret(4);
    assert!(has(&run, 1263782));
    run.app
        .init_resource::<crate::spell_cast::test_harness::ObservedTriggeredCasts>();
    cast_finisher(&mut run, 31884);
    cast_finisher(&mut run, 24275);
    wait_for_damage(&mut run, 24275);
    for _ in 0..100 {
        if damage(&run, 184575) > 0 {
            break;
        }
        run.app.update();
    }
    let launches = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedTriggeredCasts>()
        .0;
    let events = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| event.spell_id == Some(184575))
        .collect::<Vec<_>>();
    assert!(
        damage(&run, 184575) > 0,
        "actual Hammer during Wrath must launch native background Blade; launches={launches:?}, events={events:?}"
    );
}

#[test]
#[ignore = "actual Divine Toll Judgment child must earn native Holy Power"]
fn dps_parity_ret_batch_toll_judgment_funds_power() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret(4);
    assert_eq!(current_holy_power(&run), 0.0);
    run.app
        .init_resource::<crate::spell_cast::test_harness::ObservedTriggeredCasts>();
    cast_finisher(&mut run, 375576);
    wait_for_damage(&mut run, 406957);
    let events = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| event.kind == CombatLogKind::Energize || event.spell_id == Some(406957))
        .collect::<Vec<_>>();
    let launches = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedTriggeredCasts>()
        .0;
    assert_eq!(
        current_holy_power(&run),
        1.0,
        "real Toll Judgment must generate native220637 Holy Power; events={events:?}, launches={launches:?}"
    );
}
