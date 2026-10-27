//! Real selected-Sentinel target mark, recharge and Lunar Storm boundaries.
use super::marksmanship_mechanics_tests::cast;
use super::*;
use server::simc_apl::Facts;
use shared::protocol::CombatLogKind;

fn player() -> Run {
    let mut run = prepare_literal(255, 3).unwrap();
    engage(&mut run);
    run
}

fn owns_mark(run: &Run) -> bool {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(run.targets[0])
        .unwrap()
        .iter()
        .any(|aura| aura.caster == run.actor && aura.spell_id == 1253601)
}

#[test]
#[ignore = "requires private spec255 player inputs and native world DB"]
fn survival_sentinel_actual_tip_impacts_can_earn_owned_marks_without_takedown() {
    // SimC76f8118 sc_hunter.cpp1363–1366/3694–3709: actual Tip
    // consumption rolls a native Sentinel mark without forcing it through Takedown.
    let _seed = crate::random::seed_scope(70448);
    let mut run = player();
    hunter_fixture::provision_pet(run.app.world_mut(), run.actor).unwrap();
    cast(&mut run, 883, true);
    assert!(!owns_mark(&run));
    for _ in 0..32 {
        cast(&mut run, 259489, false);
        cast(&mut run, 186270, false);
        if owns_mark(&run) {
            return;
        }
    }
    panic!("32 real earned Tip impacts produced no native Sentinel mark");
}

fn bomb_full_recharge(run: &Run) -> f64 {
    use crate::simc_apl_native::{Bindings, Encounter, FightStyle, NativeFacts};
    let bindings = Bindings {
        actions: [("wildfire_bomb".into(), 259495)].into(),
        global_cooldown_spell: 259495,
        ..Default::default()
    };
    NativeFacts::new(
        run.app.world(),
        run.actor,
        &bindings,
        Encounter::Fixture {
            started_at: 0.0,
            duration: 300.0,
            style: FightStyle::Patchwerk,
        },
        3,
    )
    .value("cooldown.wildfire_bomb.full_recharge_time")
    .unwrap()
}

#[test]
#[ignore = "requires private spec255 player inputs and native world DB"]
fn survival_sentinel_actual_takedown_marks_primary_and_reduces_bomb_recharge() {
    // SimC76f8118 sc_hunter.cpp6400–6401/3665–3667: actual Takedown
    // force-marks its hostile primary and reads Moon's Blessing's native reduction.
    let mut run = player();
    cast(&mut run, 259495, false);
    cast(&mut run, 259495, false);
    let before = bomb_full_recharge(&run);
    let start = crate::spell_cast::history_now(run.app.world());
    cast(&mut run, 1250646, false);
    assert!(
        owns_mark(&run),
        "actual Takedown with Lunar Calling must force an owned native mark"
    );
    let elapsed = crate::spell_cast::history_now(run.app.world()) - start;
    let earned_reduction = before - elapsed - bomb_full_recharge(&run);
    let native_ms = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(1253825)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 2)
        .unwrap()
        .base_points;
    assert!(
        earned_reduction >= f64::from(native_ms) / 1000.0 - 0.1,
        "actual mark must reduce running native Bomb recharge: {earned_reduction}"
    );
}

#[test]
#[ignore = "requires private spec255 player inputs and native world DB"]
fn survival_sentinel_actual_bomb_consumes_mark_into_native_lunar_storm_pack_damage() {
    // SimC76f8118 sc_hunter.cpp7104–7107/3760–3773: actual Bomb impact
    // consumes its owned mark and schedules native Storm payloads on that target.
    let mut run = player();
    cast(&mut run, 1250646, false);
    assert!(owns_mark(&run));
    cast(&mut run, 259495, false);
    for _ in 0..80 {
        run.app.update();
    }
    assert!(
        !owns_mark(&run),
        "the actual Bomb primary must consume its earlier native mark"
    );
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    for &target in &run.targets {
        assert!(
            events
                .iter()
                .any(|event| event.source == Some(run.actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(1253733)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0),
            "native Lunar Storm must actually damage in-radius enemy {target:?}"
        );
    }
}
