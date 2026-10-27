//! Actual Kill Command shortens running Wildfire Bomb recovery by selected Wildfire Infusion.
use super::marksmanship_mechanics_tests::cast;
use super::*;

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
        1,
    )
    .value("cooldown.wildfire_bomb.full_recharge_time")
    .unwrap()
}

#[test]
#[ignore = "requires private spec255 player inputs and native world DB"]
fn survival_native_kill_command_reduces_running_bomb_recharge_by_wildfire_infusion() {
    // SimC76f8118 sc_hunter.cpp6622/7776: every actual Kill Command execute
    // adjusts Wildfire Bomb's running recovery by selected460198 effect1 once.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(255, 1).unwrap();
    hunter_fixture::provision_pet(run.app.world_mut(), run.actor).unwrap();
    cast(&mut run, 883, true);
    engage(&mut run);
    cast(&mut run, 259495, false);
    cast(&mut run, 259495, false);
    let before = bomb_full_recharge(&run);
    let start = crate::spell_cast::history_now(run.app.world());
    cast(&mut run, 259489, false);
    let elapsed = crate::spell_cast::history_now(run.app.world()) - start;
    let earned_reduction = before - elapsed - bomb_full_recharge(&run);
    let native_seconds = f64::from(
        run.app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(460198)
            .unwrap()
            .effects
            .iter()
            .find(|effect| effect.index == 0)
            .unwrap()
            .base_points,
    ) / 1000.0;
    assert!(
        (earned_reduction - native_seconds).abs() < 0.1,
        "one actual Kill Command must reduce running Bomb recovery by {native_seconds}s: {earned_reduction}"
    );
}
