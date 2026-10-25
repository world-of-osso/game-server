use super::super::literal_rotation::{
    Driver,
    equipment::Equipment,
    execution::{self, Prepared},
};
use super::tests::facts;
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use server::simc_apl::{Action, Facts};
use shared::protocol::CombatLogKind;

fn action(name: &str) -> Action {
    Action {
        source_row: None,
        name: name.into(),
        target: None,
        options: Default::default(),
    }
}

#[test]
#[ignore = "native Ret literal action: shared knowledge resolves selected FV and casts real damage"]
fn dps_parity_ret_literal_native_prepared_verdict_uses_selected_damage_action() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::super::ret_tier_tests::native_ret(4);
    let mut bindings = bindings();
    assert!(
        facts(&run, &bindings)
            .value("cooldown.judgment.charges")
            .unwrap()
            >= 1.0
    );
    super::super::ret_purpose_tests::earn_finisher_power(&mut run, 383328);
    let equipment = Equipment::load(run.app.world(), run.actor, &mut bindings).unwrap();
    let mut dispatched = false;
    for _ in 0..200 {
        let prepared = Prepared::read(
            run.app.world_mut(),
            run.actor,
            &run.targets,
            &equipment,
            execution_actions(),
            false,
        )
        .unwrap();
        if let Some(execution) = prepared
            .execution(&action("templars_verdict"), &equipment)
            .unwrap()
        {
            execution::execute(run.app.world_mut(), run.actor, execution).unwrap();
            dispatched = true;
            break;
        }
        run.app.update();
    }
    assert!(
        dispatched,
        "naturally funded literal Verdict must become ready"
    );
    for _ in 0..200 {
        run.app.update();
        if damage(&run, 383328) > 0 {
            break;
        }
    }
    assert!(
        damage(&run, 383328) > 0,
        "selected Final Verdict must deal actual damage"
    );
    assert_eq!(damage(&run, 224266), 0, "base Templar damage must not leak");
    assert!(
        run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.spell_id == Some(383328)
                    && event.kind == CombatLogKind::CastSuccess
            })
    );
}

fn cast_literal_storm(run: &mut super::super::Run) {
    let mut bindings = bindings();
    let equipment = Equipment::load(run.app.world(), run.actor, &mut bindings).unwrap();
    let action = action("divine_storm");
    for _ in 0..200 {
        let prepared = Prepared::read(
            run.app.world_mut(),
            run.actor,
            &run.targets,
            &equipment,
            execution_actions(),
            false,
        )
        .unwrap();
        if let Some(execution) = prepared.execution(&action, &equipment).unwrap() {
            execution::execute(run.app.world_mut(), run.actor, execution).unwrap();
            return;
        }
        run.app.update();
    }
    panic!("naturally funded literal Storm did not become ready");
}

#[test]
#[ignore = "native literal self-cast must retain hostile primary for forwarded child"]
fn dps_parity_ret_tier_literal_self_cast_preserves_arbiter_hostile_primary() {
    let _seed = crate::random::seed_scope(70448);
    assert_arbiter_primary(super::super::ret_tier_tests::native_ret(4));
}

#[test]
#[ignore = "native Player self-cast retains its selected hostile primary"]
fn dps_parity_ret_tier_player_self_cast_preserves_arbiter_hostile_primary() {
    let _seed = crate::random::seed_scope(70448);
    let (profile, traits) = super::super::read_reference(70).unwrap();
    let mut run = super::super::prepare_reference(
        profile,
        &traits,
        1,
        crate::followers::simulator::ParityActor::Player { race: 3 },
    )
    .unwrap();
    super::super::engage(&mut run);
    assert_arbiter_primary(run);
}

fn assert_arbiter_primary(mut run: super::super::Run) {
    use super::super::ret_tier_tests::{cast_finisher, grant_native_purpose, has};
    use crate::spell_cast::test_harness::ObservedTriggeredCasts;
    const FINAL_VERDICT: u32 = 383328;
    const DIVINE_STORM: u32 = 53385;
    const DIVINE_ARBITER: u32 = 1306923;
    const VERDICT_ARBITER: u32 = 1306161;
    run.app.init_resource::<ObservedTriggeredCasts>();
    execution::execute(
        run.app.world_mut(),
        run.actor,
        execution::Execution::AutoAttack(run.targets[0]),
    )
    .unwrap();
    grant_native_purpose(&mut run);
    cast_finisher(&mut run, FINAL_VERDICT);
    assert!(has(&run, VERDICT_ARBITER));
    assert_eq!(damage(&run, DIVINE_ARBITER), 0);
    super::super::ret_purpose_tests::earn_finisher_power(&mut run, DIVINE_STORM);
    cast_literal_storm(&mut run);
    assert!(
        !has(&run, VERDICT_ARBITER),
        "real opposite-kind spender consumed pending kind"
    );
    let child_targets = run
        .app
        .world()
        .resource::<ObservedTriggeredCasts>()
        .0
        .iter()
        .filter(|(caster, spell, _)| *caster == run.actor && *spell == DIVINE_ARBITER)
        .map(|(_, _, target)| *target)
        .collect::<Vec<_>>();
    assert_eq!(
        child_targets,
        vec![Some(run.targets[0])],
        "literal self-target must not replace forwarded child's hostile primary; caster={:?}",
        run.actor
    );
    for _ in 0..200 {
        run.app.update();
        if damage(&run, DIVINE_ARBITER) > 0 {
            break;
        }
    }
    assert!(
        run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(DIVINE_ARBITER)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0),
        "native child must actually damage hostile primary"
    );
}

fn damage(run: &super::super::Run, spell: u32) -> i64 {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(spell)
                && event.target == Some(run.targets[0].to_bits())
        })
        .filter(|event| event.kind == CombatLogKind::Damage)
        .map(|event| i64::from(event.amount))
        .sum()
}

#[test]
#[ignore = "native Ret literal source: pinned precombat first row exports actual native stats"]
fn dps_parity_ret_literal_native_source_snapshot_exports_real_stats() {
    let mut run = super::super::ret_tier_tests::native_ret(4);
    let mut bindings = bindings();
    let equipment = Equipment::load(run.app.world(), run.actor, &mut bindings).unwrap();
    let prepared = Prepared::read(
        run.app.world_mut(),
        run.actor,
        &run.targets,
        &equipment,
        execution_actions(),
        true,
    )
    .unwrap();
    let expected = serde_json::json!(
        run.app
            .world()
            .get::<crate::stats::CharacterStats>(run.actor)
            .unwrap()
            .primary
            .strength
    );
    let mut driver = Driver::new(read_program().unwrap());
    let (action, execution) = driver
        .select("precombat", &facts(&run, &bindings), &prepared, &equipment)
        .unwrap()
        .expect("pinned Ret precombat must select its native snapshot");
    driver
        .execute(run.app.world_mut(), run.actor, action, execution)
        .unwrap();
    let snapshot = driver
        .snapshot
        .expect("actual snapshot execution must export state");
    assert_eq!(snapshot["attribute"]["strength"], expected);
    assert_eq!(snapshot["level"], 90);
}
