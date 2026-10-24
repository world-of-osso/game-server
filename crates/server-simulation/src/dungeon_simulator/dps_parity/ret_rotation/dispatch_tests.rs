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
