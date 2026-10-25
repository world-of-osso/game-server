//! Literal row options gate actual native actor readiness, never replace it.
//! Configured fixture distance is independent of the native protocol moving bit.
use super::*;
use execution::NativeAction;
use std::collections::BTreeMap;

type Run = super::super::Run;

fn native_monk() -> (Run, Equipment) {
    assert_eq!(std::env::var("DPS_PARITY_ACTOR").unwrap(), "player");
    let run = super::super::prepare_literal(269, 1).unwrap();
    assert!(
        run.app
            .world()
            .get::<shared::components::Player>(run.actor)
            .is_some()
    );
    let equipment = Equipment::load(
        run.app.world(),
        run.actor,
        &mut crate::simc_apl_native::Bindings::default(),
    )
    .unwrap();
    (run, equipment)
}

fn prepare_native(run: &mut Run, equipment: &Equipment) -> Prepared {
    Prepared::read(
        run.app.world_mut(),
        run.actor,
        &run.targets,
        equipment,
        [NativeAction {
            name: "tiger_palm",
            spell: 100780,
            talent: None,
            self_target: false,
        }],
        false,
    )
    .unwrap()
}

fn select(
    literal: &str,
    prepared: &Prepared,
    equipment: &Equipment,
) -> Option<(Action, Execution)> {
    let program = Program::parse(&[("default", literal)]).unwrap();
    Driver::new(program)
        .select(
            "default",
            &BTreeMap::<String, f64>::new(),
            prepared,
            equipment,
        )
        .unwrap()
}

fn assert_native_ready(name: &str, prepared: &Prepared, equipment: &Equipment) {
    let action = Action {
        source_row: None,
        name: name.into(),
        target: None,
        options: Default::default(),
    };
    assert!(
        prepared.execution(&action, equipment).unwrap().is_some(),
        "native {name} admission premise failed; not a row-option RED"
    );
}

#[test]
#[ignore = "native269 row-option RED; requires current private inputs and player identity"]
fn literal_native_gating_moving_one_rejects_a_stationary_native_actor() {
    let _seed = crate::random::seed_scope(70448);
    let (mut run, equipment) = native_monk();
    assert!(
        !run.app
            .world()
            .get::<crate::networking::Moving>(run.actor)
            .unwrap()
            .0
    );
    let prepared = prepare_native(&mut run, &equipment);
    assert_native_ready("tiger_palm", &prepared, &equipment);
    assert!(select("tiger_palm,moving=0", &prepared, &equipment).is_some());
    assert!(
        select("tiger_palm,moving=1", &prepared, &equipment).is_none(),
        "moving=1 must not silently execute while stationary"
    );
}

#[test]
#[ignore = "native269 row-option RED; requires current private inputs and player identity"]
fn literal_native_gating_moving_zero_rejects_native_moving_input() {
    let _seed = crate::random::seed_scope(70448);
    let (mut run, equipment) = native_monk();
    // The native protocol movement bit is the input under test, not a position workaround.
    run.app
        .world_mut()
        .get_mut::<crate::networking::Moving>(run.actor)
        .unwrap()
        .0 = true;
    let prepared = prepare_native(&mut run, &equipment);
    assert_native_ready("tiger_palm", &prepared, &equipment);
    assert!(select("tiger_palm,moving=1", &prepared, &equipment).is_some());
    assert!(
        select("tiger_palm,moving=0", &prepared, &equipment).is_none(),
        "moving=0 must honor actual native movement"
    );
}

fn start_native_gcd(run: &mut Run) {
    execution::execute(
        run.app.world_mut(),
        run.actor,
        Execution::Spell(100780, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    let history = run
        .app
        .world()
        .get::<crate::spell_cast::SpellHistory>(run.actor)
        .unwrap();
    assert!(
        history.has_active_gcd(crate::spell_cast::history_now(run.app.world())),
        "a real native Tiger Palm must start the Player GCD"
    );
}

#[test]
#[ignore = "native269 row-option RED; requires current private inputs and player identity"]
fn literal_native_gating_during_gcd_requires_explicit_off_gcd_row_permission() {
    let _seed = crate::random::seed_scope(70448);
    let (mut run, equipment) = native_monk();
    start_native_gcd(&mut run);
    let prepared = prepare_native(&mut run, &equipment);
    assert_native_ready("auto_attack", &prepared, &equipment);
    assert!(
        select("auto_attack", &prepared, &equipment).is_none(),
        "native off-GCD readiness alone must not ignore the literal row's scheduling permission"
    );
}

#[test]
#[ignore = "native269 off-GCD positive control; requires current private inputs and player identity"]
fn literal_native_gating_explicit_off_gcd_row_preserves_native_gcd() {
    let _seed = crate::random::seed_scope(70448);
    let (mut run, equipment) = native_monk();
    start_native_gcd(&mut run);
    let prepared = prepare_native(&mut run, &equipment);
    assert_native_ready("auto_attack", &prepared, &equipment);
    let mut driver =
        Driver::new(Program::parse(&[("default", "auto_attack,use_off_gcd=1")]).unwrap());
    let (action, execution) = driver
        .select(
            "default",
            &BTreeMap::<String, f64>::new(),
            &prepared,
            &equipment,
        )
        .unwrap()
        .expect("explicit off-GCD row must retain native ready auto attack");
    driver
        .execute(run.app.world_mut(), run.actor, action, execution)
        .unwrap();
    assert!(
        run.app
            .world()
            .get::<crate::combat::AutoAttack>(run.actor)
            .is_some()
    );
    let history = run
        .app
        .world()
        .get::<crate::spell_cast::SpellHistory>(run.actor)
        .unwrap();
    assert!(
        history.has_active_gcd(crate::spell_cast::history_now(run.app.world())),
        "row permission must not reset or waive the native GCD"
    );
}

#[test]
#[ignore = "native269 configured movement row and missing-fact propagation; player identity"]
fn literal_native_gating_configured_movement_distance_drives_literal_row() {
    let _seed = crate::random::seed_scope(70448);
    let (mut run, equipment) = native_monk();
    let prepared = prepare_native(&mut run, &equipment);
    assert_native_ready("tiger_palm", &prepared, &equipment);
    let bindings = crate::simc_apl_native::Bindings::default();
    let encounter = Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    for (distance, eligible) in [(0.0, false), (7.25, true)] {
        run.app
            .world_mut()
            .get_mut::<crate::simc_apl_native::FixtureMovement>(run.actor)
            .unwrap()
            .distance_to_move = distance;
        let facts = crate::simc_apl_native::NativeFacts::new(
            run.app.world(),
            run.actor,
            &bindings,
            encounter,
            1,
        );
        assert_eq!(facts.value("movement.distance").unwrap(), distance);
        let mut driver = Driver::new(
            Program::parse(&[("default", "tiger_palm,if=movement.distance>6")]).unwrap(),
        );
        let selected = driver
            .select("default", &facts, &prepared, &equipment)
            .unwrap();
        assert_eq!(
            selected.is_some(),
            eligible,
            "configured distance{distance}"
        );
        if let Some((action, _)) = selected {
            assert_eq!(action.name, "tiger_palm");
        }
    }
    run.app
        .world_mut()
        .entity_mut(run.actor)
        .remove::<crate::simc_apl_native::FixtureMovement>();
    let facts = crate::simc_apl_native::NativeFacts::new(
        run.app.world(),
        run.actor,
        &bindings,
        encounter,
        1,
    );
    let mut driver =
        Driver::new(Program::parse(&[("default", "tiger_palm,if=movement.distance>6")]).unwrap());
    let error = match driver.select("default", &facts, &prepared, &equipment) {
        Err(error) => error,
        Ok(_) => panic!("missing fixture movement configuration must propagate its named error"),
    };
    assert!(
        error.contains("NATIVE_MOVEMENT_DISTANCE_UNAVAILABLE"),
        "{error}"
    );
}
