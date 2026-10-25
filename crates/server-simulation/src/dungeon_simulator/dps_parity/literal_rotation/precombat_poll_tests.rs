//! Shared native Fire boundary; full88-row FireFacts/adapter remains14779-owned.
use super::*;
use execution::NativeAction;
use shared::protocol::CombatLogKind;
use std::collections::BTreeMap;

type Run = super::super::Run;
const PYROBLAST: u32 = 11366;
const ACTIONS: &[NativeAction<'static>] = &[
    NativeAction {
        name: "arcane_intellect",
        spell: 1459,
        talent: None,
        self_target: true,
    },
    NativeAction {
        name: "mirror_image",
        spell: 55342,
        talent: None,
        self_target: true,
    },
    NativeAction {
        name: "pyroblast",
        spell: PYROBLAST,
        talent: None,
        self_target: false,
    },
    NativeAction {
        name: "fire_blast",
        spell: 108853,
        talent: None,
        self_target: false,
    },
    NativeAction {
        name: "combustion",
        spell: 190319,
        talent: None,
        self_target: true,
    },
];

fn native_fire() -> (Run, Equipment) {
    assert_eq!(std::env::var("DPS_PARITY_ACTOR").unwrap(), "player");
    let run = super::super::prepare_literal(63, 1).unwrap();
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
        ACTIONS.iter().copied(),
        true,
    )
    .unwrap()
}

fn action(name: &str) -> Action {
    Action {
        name: name.into(),
        target: None,
        options: Default::default(),
    }
}

fn assert_ready(name: &str, prepared: &Prepared, equipment: &Equipment) {
    assert!(
        prepared
            .execution(&action(name), equipment)
            .unwrap()
            .is_some(),
        "native {name} readiness premise failed; not a cursor/options RED"
    );
}

fn pyro_flight(run: &Run, started_at: f64, property: &str) -> f64 {
    let mut bindings = crate::simc_apl_native::Bindings::default();
    bindings.actions.insert("pyroblast".into(), PYROBLAST);
    let encounter = Encounter::Fixture {
        started_at,
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    crate::simc_apl_native::NativeFacts::new(run.app.world(), run.actor, &bindings, encounter, 1)
        .value(&format!("action.pyroblast.{property}"))
        .unwrap()
}

fn own_casts(run: &Run) -> Vec<u32> {
    run.app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits()) && event.kind == CombatLogKind::CastSuccess
        })
        .filter_map(|event| event.spell_id)
        .filter(|spell| [1459, 55342, PYROBLAST].contains(spell))
        .collect()
}

#[test]
#[ignore = "native63 Player/current private view; shared precombat poll RED"]
fn literal_precombat_poll_resumes_native_fire_rows_after_gcd_and_hardcast() {
    let _seed = crate::random::seed_scope(70448);
    let (mut run, equipment) = native_fire();
    let prepared = prepare_native(&mut run, &equipment);
    for name in ["arcane_intellect", "mirror_image", "pyroblast"] {
        assert_ready(name, &prepared, &equipment);
    }
    // Actual Fire cast rows stay plain. Constant/reset/set rows isolate the shared
    // cursor/variable contract without inventing FireFacts for the full source.
    let program = Program::parse(&[
        ("precombat", "arcane_intellect"),
        ("precombat", "variable,name=cast_remains_time,value=0.2"),
        (
            "precombat",
            "variable,name=flamestriking,op=reset,default=1",
        ),
        (
            "precombat",
            "variable,name=flamestriking,op=set,value=variable.flamestriking",
        ),
        (
            "precombat",
            "snapshot_stats,if=variable.flamestriking=1&variable.cast_remains_time=0.2",
        ),
        ("precombat", "mirror_image"),
        ("precombat", "pyroblast"),
    ])
    .unwrap();
    let mut driver = Driver::new(program);
    let facts = BTreeMap::<String, f64>::new();
    let PrecombatPoll::Selected { action, execution } = driver
        .poll_precombat(&facts, &prepared, &equipment)
        .unwrap()
    else {
        panic!("native Arcane Intellect must be first");
    };
    assert_eq!(action.name, "arcane_intellect");
    driver
        .execute(run.app.world_mut(), run.actor, action, execution)
        .unwrap();
    let history = run
        .app
        .world()
        .get::<crate::spell_cast::SpellHistory>(run.actor)
        .unwrap();
    assert!(
        history.has_active_gcd(crate::spell_cast::history_now(run.app.world())),
        "actual Arcane Intellect must start native GCD"
    );
    let prepared = prepare_native(&mut run, &equipment);
    let busy_poll = driver
        .poll_precombat(&facts, &prepared, &equipment)
        .unwrap();
    assert_eq!(
        std::mem::discriminant(&busy_poll),
        std::mem::discriminant(&PrecombatPoll::Waiting),
        "busy native GCD must not consume the remaining source rows or return Finished"
    );
    let before = crate::spell_cast::now(run.app.world());
    let mut pyro_duration = None;
    let mut finished = false;
    for _ in 0..600 {
        let prepared = prepare_native(&mut run, &equipment);
        match driver
            .poll_precombat(&facts, &prepared, &equipment)
            .unwrap()
        {
            PrecombatPoll::Waiting => {
                run.app.update();
            }
            PrecombatPoll::Selected { action, execution } => {
                let is_pyro = action.name == "pyroblast";
                if is_pyro {
                    assert!(
                        action.options.is_empty(),
                        "plain source Pyroblast has no invented precast/travel option"
                    );
                }
                driver
                    .execute(run.app.world_mut(), run.actor, action, execution)
                    .unwrap();
                if is_pyro {
                    let cast = run
                        .app
                        .world()
                        .get::<shared::casting::CastState>(run.actor)
                        .expect("ordinary Pyroblast must really hardcast");
                    assert_eq!(cast.spell_id, PYROBLAST);
                    assert!(cast.duration > 0.0);
                    pyro_duration = Some(f64::from(cast.duration));
                    assert_eq!(
                        pyro_flight(&run, before, "in_flight"),
                        0.0,
                        "hardcast before launch is not native flight"
                    );
                }
            }
            PrecombatPoll::Finished => {
                finished = true;
                break;
            }
        }
    }
    assert!(
        finished,
        "precombat must finish through bounded real App frames"
    );
    assert!(
        driver.snapshot.is_some(),
        "source variable/reset/set rows must permit the snapshot before Mirror Image"
    );
    let elapsed = crate::spell_cast::now(run.app.world()) - before;
    assert!(elapsed + 0.001 >= pyro_duration.expect("native Pyroblast was never reached"));
    assert!(
        run.app
            .world()
            .get::<shared::casting::CastState>(run.actor)
            .is_none()
    );
    let history = run
        .app
        .world()
        .get::<crate::spell_cast::SpellHistory>(run.actor)
        .unwrap();
    assert!(!history.has_active_gcd(crate::spell_cast::history_now(run.app.world())));
    assert_eq!(
        own_casts(&run),
        [1459, 55342, PYROBLAST],
        "no skipped/replayed source cast"
    );
    let epoch = crate::spell_cast::now(run.app.world());
    let flight = pyro_flight(&run, epoch, "in_flight");
    let remaining = pyro_flight(&run, epoch, "in_flight_remains");
    assert!(flight == 0.0 || flight == 1.0);
    assert!(remaining >= 0.0);
    let bindings = crate::simc_apl_native::Bindings::default();
    let encounter = Encounter::Fixture {
        started_at: epoch,
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let clock = crate::simc_apl_native::NativeFacts::new(
        run.app.world(),
        run.actor,
        &bindings,
        encounter,
        1,
    );
    assert_eq!(
        clock.value("time").unwrap(),
        0.0,
        "caller starts300s fight epoch after real setup"
    );
    let output = super::super::inputs::directory()
        .unwrap()
        .join("native-fire-precombat-poll.json");
    std::fs::write(output, serde_json::to_vec_pretty(&serde_json::json!({"casts":own_casts(&run),"setup_elapsed":elapsed,"pyro_cast_duration":pyro_duration,"flight_at_finished":flight,"flight_remains_at_finished":remaining})).unwrap()).unwrap();
}

#[test]
#[ignore = "native63 Player/current private view; shared while-casting row-option RED"]
fn literal_precombat_options_require_while_casting_permission_without_interrupting_pyroblast() {
    let _seed = crate::random::seed_scope(70448);
    let (mut run, equipment) = native_fire();
    execution::execute(
        run.app.world_mut(),
        run.actor,
        Execution::Spell(PYROBLAST, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    let before = run
        .app
        .world()
        .get::<shared::casting::CastState>(run.actor)
        .expect("real hardcast Pyroblast required")
        .clone();
    assert!(before.duration > 0.0);
    let prepared = prepare_native(&mut run, &equipment);
    assert_ready("fire_blast", &prepared, &equipment);
    assert_ready("combustion", &prepared, &equipment);
    let facts = BTreeMap::<String, f64>::new();
    let mut unpermitted =
        Driver::new(Program::parse(&[("default", "fire_blast,use_off_gcd=1")]).unwrap());
    assert!(
        unpermitted
            .select("default", &facts, &prepared, &equipment)
            .unwrap()
            .is_none(),
        "native cast permission alone must not ignore missing literal use_while_casting"
    );
    for name in ["fire_blast", "combustion"] {
        let prepared = prepare_native(&mut run, &equipment);
        let literal = format!("{name},use_off_gcd=1,use_while_casting=1");
        let mut permitted = Driver::new(Program::parse(&[("default", &literal)]).unwrap());
        let (action, execution) = permitted
            .select("default", &facts, &prepared, &equipment)
            .unwrap()
            .expect("explicit row and native while-casting permission required");
        assert_eq!(
            action.options.get("use_while_casting").map(String::as_str),
            Some("1")
        );
        permitted
            .execute(run.app.world_mut(), run.actor, action, execution)
            .unwrap();
        let after = run
            .app
            .world()
            .get::<shared::casting::CastState>(run.actor)
            .expect("native instant must not interrupt the Pyroblast hardcast");
        assert_eq!(
            (after.spell_id, after.duration, after.elapsed),
            (before.spell_id, before.duration, before.elapsed)
        );
    }
}
