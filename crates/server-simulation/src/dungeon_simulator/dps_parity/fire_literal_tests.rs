//! Real native follower setup and generic facts used by the pinned Fire source.
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires native70448 private DB and exact Fire63 reference inputs"]
fn fire_literal_follower_executes_ordered_precombat_before_pull() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(63, 1).expect("native Fire setup");
    let casts: Vec<_> = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits()) && event.kind == CombatLogKind::CastSuccess
        })
        .filter_map(|event| event.spell_id)
        .collect();
    let index = |spell| {
        casts
            .iter()
            .position(|id| *id == spell)
            .unwrap_or_else(|| panic!("missing precombat spell{spell}: {casts:?}"))
    };
    assert!(index(1459) < index(55342) && index(55342) < index(11366));
    engage(&mut run);
    let started_at = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started_at);
    for _ in 0..100 {
        drive_player(&mut run).expect("literal Fire combat action");
        run.app.update();
    }
    assert!(
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(133)
                && event.kind == CombatLogKind::CastSuccess),
        "literal Fire filler must cast Fireball"
    );
}

fn fire_native_fact(run: &Run, name: &str) -> Result<f64, String> {
    use crate::simc_apl_native::{Bindings, Encounter, FightStyle, NativeFacts};
    use server::simc_apl::{Action, ActionFacts};
    let bindings = Bindings {
        actions: [("fireball".into(), 133), ("fire_blast".into(), 108853)].into(),
        global_cooldown_spell: 133,
        ..default()
    };
    let encounter = Encounter::Fixture {
        started_at: 0.0,
        duration: 300.0,
        style: FightStyle::Patchwerk,
    };
    NativeFacts::new(run.app.world(), run.actor, &bindings, encounter, 1)
        .with_targets(run.targets.first().copied(), &run.targets)
        .value_for(
            &Action {
                name: "fire_blast".into(),
                source_row: None,
                target: None,
                options: Default::default(),
            },
            name,
        )
}

fn start_fire_native_cast(run: &mut Run, spell: u32) {
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        spell,
        Some(run.targets[0]),
        false,
    )
    .expect("ordinary native Fire cast");
}

#[test]
#[ignore = "requires native70448 private DB and exact Fire63 reference inputs"]
fn fire_native_action_execution_facts_follow_real_cast_progress() {
    let mut run = prepare_literal(63, 1).unwrap();
    assert_eq!(fire_native_fact(&run, "action.fireball.executing"), Ok(0.0));
    assert_eq!(
        fire_native_fact(&run, "action.fireball.execute_remains"),
        Ok(0.0)
    );
    start_fire_native_cast(&mut run, 133);
    assert_eq!(fire_native_fact(&run, "action.fireball.executing"), Ok(1.0));
    let before = fire_native_fact(&run, "action.fireball.execute_remains").unwrap();
    assert!(before > 0.0);
    run.app.update();
    assert!(fire_native_fact(&run, "action.fireball.execute_remains").unwrap() < before);
    assert_eq!(
        fire_native_fact(&run, "action.fire_blast.executing"),
        Ok(0.0)
    );
    preparation::finish_pending_setup(&mut run).unwrap();
    assert_eq!(fire_native_fact(&run, "action.fireball.executing"), Ok(0.0));
    assert_eq!(
        fire_native_fact(&run, "action.fireball.execute_remains"),
        Ok(0.0)
    );
}

#[test]
#[ignore = "requires native70448 private DB and exact Fire63 reference inputs"]
fn fire_native_last_used_changes_only_at_real_successful_completion() {
    let mut run = prepare_literal(63, 1).unwrap();
    let unused = fire_native_fact(&run, "action.fireball.last_used").unwrap();
    assert!(unused < 0.0);
    start_fire_native_cast(&mut run, 133);
    assert_eq!(
        fire_native_fact(&run, "action.fireball.last_used"),
        Ok(unused)
    );
    while run
        .app
        .world()
        .get::<shared::casting::CastState>(run.actor)
        .is_some()
    {
        run.app.update();
    }
    let completed = crate::spell_cast::now(run.app.world());
    assert_eq!(
        fire_native_fact(&run, "action.fireball.last_used"),
        Ok(completed)
    );
    run.app.update();
    assert_eq!(
        fire_native_fact(&run, "action.fireball.last_used"),
        Ok(completed)
    );
    assert!(fire_native_fact(&run, "action.fire_blast.last_used").unwrap() < 0.0);
}

#[test]
#[ignore = "requires native70448 private DB and exact Fire63 reference inputs"]
fn fire_native_cooldown_react_respects_real_recovery_even_with_spare_charges() {
    let mut run = prepare_literal(63, 1).unwrap();
    assert_eq!(fire_native_fact(&run, "cooldown_react"), Ok(1.0));
    start_fire_native_cast(&mut run, 108853);
    assert_eq!(fire_native_fact(&run, "cooldown_react"), Ok(0.0));
    for _ in 0..12 {
        run.app.update();
    }
    assert_eq!(fire_native_fact(&run, "cooldown_react"), Ok(1.0));
}
