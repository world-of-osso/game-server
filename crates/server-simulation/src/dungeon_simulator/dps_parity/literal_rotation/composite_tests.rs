//! Ordered native plans only; completion/timing belongs to the precombat executor.
use super::*;
use execution::{NativeAction, NativeComposite};
use shared::components::Player;
use std::collections::BTreeMap;

type Run = super::super::Run;
const COATINGS: [u32; 4] = [2823, 3408, 381664, 381637];
const STEPS: [NativeAction<'static>; 4] = [
    NativeAction {
        name: "deadly_poison",
        spell: 2823,
        talent: None,
        self_target: true,
    },
    NativeAction {
        name: "crippling_poison",
        spell: 3408,
        talent: None,
        self_target: true,
    },
    NativeAction {
        name: "amplifying_poison",
        spell: 381664,
        talent: None,
        self_target: true,
    },
    NativeAction {
        name: "atrophic_poison",
        spell: 381637,
        talent: None,
        self_target: true,
    },
];

fn native_player(spec: u32) -> (Run, Equipment) {
    let (profile, traits) = super::super::read_reference(spec).unwrap();
    let mut run = super::super::prepare_reference(
        profile,
        &traits,
        1,
        crate::followers::simulator::ParityActor::Player { race: 1 },
    )
    .unwrap();
    let player = run.app.world().get::<Player>(run.actor).unwrap();
    assert_eq!(
        (player.class, player.race),
        (if spec == 259 { 4 } else { 1 }, 1)
    );
    let encounter = Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    capture_fixture(run.app.world_mut(), run.actor, encounter).unwrap();
    let equipment = Equipment::load(
        run.app.world(),
        run.actor,
        &mut crate::simc_apl_native::Bindings::default(),
    )
    .unwrap();
    (run, equipment)
}

fn owned_aura(run: &Run, spell: u32) -> bool {
    let now = crate::spell_cast::now(run.app.world());
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .is_some_and(|auras| {
            auras.iter().any(|aura| {
                aura.spell_id == spell
                    && aura.caster == run.actor
                    && aura.expires_at.is_none_or(|at| at > now)
            })
        })
}

fn cast_coating(run: &mut Run, spell: u32) {
    for _ in 0..200 {
        match crate::spell_cast::cast_for_player_with_target(
            run.app.world_mut(),
            run.actor,
            spell,
            Some(run.actor),
        ) {
            Ok(()) => {
                assert!(
                    run.app
                        .world()
                        .get::<shared::casting::CastState>(run.actor)
                        .is_some_and(|cast| cast.duration > 0.0),
                    "native coating must really cast"
                );
                super::super::preparation::finish_pending_setup(run).unwrap();
                assert!(
                    owned_aura(run, spell),
                    "native coating{spell} did not finish/apply"
                );
                return;
            }
            Err(
                crate::spell_cast::CastFailure::OnGlobalCooldown
                | crate::spell_cast::CastFailure::InProgress,
            ) => run.app.update(),
            Err(error) => panic!("native coating{spell} setup refused: {error:?}"),
        }
    }
    panic!("native coating{spell} never reached readiness");
}

fn prepare(
    run: &mut Run,
    equipment: &Equipment,
    name: &str,
    steps: &[NativeAction<'_>],
) -> Prepared {
    Prepared::read(
        run.app.world_mut(),
        run.actor,
        &[],
        equipment,
        steps.iter().copied(),
        false,
    )
    .unwrap()
    .with_composites(
        run.app.world_mut(),
        run.actor,
        &[],
        [NativeComposite { name, steps }],
    )
    .unwrap()
}

fn assert_native_steps_ready(
    prepared: &Prepared,
    equipment: &Equipment,
    steps: &[NativeAction<'_>],
    actor: Entity,
) {
    for step in steps {
        let action = Action {
            name: step.name.into(),
            target: None,
            options: Default::default(),
        };
        assert!(
            matches!(prepared.execution(&action, equipment).unwrap(), Some(Execution::Spell(spell, target, _))
            if spell == step.spell && target == actor),
            "each selected step must have real native admission: {}",
            step.name
        );
    }
}

fn select(prepared: &Prepared, equipment: &Equipment, name: &str) -> Option<(Action, Execution)> {
    let program = Program::parse(&[("precombat", name)]).unwrap();
    Driver::new(program)
        .select(
            "precombat",
            &BTreeMap::<String, f64>::new(),
            prepared,
            equipment,
        )
        .unwrap()
}

fn pending(selected: Option<(Action, Execution)>, actor: Entity) -> Vec<u32> {
    let (_, Execution::Composite(steps)) = selected.expect("native composite must be selected")
    else {
        panic!("selection must return an ordered pending plan, not a single cast");
    };
    steps
        .into_iter()
        .map(|step| match step {
            Execution::Spell(spell, target, _) => {
                assert_eq!(target, actor);
                spell
            }
            _ => panic!("native coating plan contains a non-spell step"),
        })
        .collect()
}

#[test]
#[ignore = "native ordered composite RED; requires private259 inputs"]
fn literal_composite_bare_poison_returns_all_four_native_ready_steps_without_casting() {
    let (mut run, equipment) = native_player(259);
    assert!(COATINGS.iter().all(|&spell| !owned_aura(&run, spell)));
    let prepared = prepare(&mut run, &equipment, "apply_poison", &STEPS);
    assert_native_steps_ready(&prepared, &equipment, &STEPS, run.actor);
    assert_eq!(
        pending(select(&prepared, &equipment, "apply_poison"), run.actor),
        COATINGS
    );
    assert!(
        run.app
            .world()
            .get::<shared::casting::CastState>(run.actor)
            .is_none()
    );
    assert!(
        COATINGS.iter().all(|&spell| !owned_aura(&run, spell)),
        "selection cannot apply coatings"
    );
}

#[test]
#[ignore = "native ordered composite RED; requires private259 inputs"]
fn literal_composite_existing_native_coating_is_skipped_without_reordering() {
    let (mut run, equipment) = native_player(259);
    cast_coating(&mut run, COATINGS[0]);
    assert!(owned_aura(&run, COATINGS[0]));
    let prepared = prepare(&mut run, &equipment, "apply_poison", &STEPS);
    assert_native_steps_ready(&prepared, &equipment, &STEPS[1..], run.actor);
    assert_eq!(
        pending(select(&prepared, &equipment, "apply_poison"), run.actor),
        COATINGS[1..]
    );
}

#[test]
#[ignore = "native ordered composite control; requires private259 inputs"]
fn literal_composite_all_active_native_steps_are_not_ready() {
    let (mut run, equipment) = native_player(259);
    cast_coating(&mut run, COATINGS[0]);
    let prepared = prepare(&mut run, &equipment, "single_coating", &STEPS[..1]);
    assert!(select(&prepared, &equipment, "single_coating").is_none());
}

#[test]
#[ignore = "native ordered composite control; requires private259 inputs"]
fn literal_composite_one_unknown_selected_step_cannot_be_silently_omitted() {
    let (mut run, equipment) = native_player(259);
    let missing = COATINGS[3];
    let known = run
        .app
        .world()
        .get::<crate::class_progression::KnownSpells>(run.actor)
        .unwrap();
    assert!(
        known.contains(missing),
        "start with the real selected native spellbook"
    );
    let retained = known.iter().filter(|&spell| spell != missing).collect();
    run.app
        .world_mut()
        .entity_mut(run.actor)
        .insert(crate::class_progression::KnownSpells::fixture(retained));
    let prepared = prepare(&mut run, &equipment, "apply_poison", &STEPS);
    assert_native_steps_ready(&prepared, &equipment, &STEPS[..3], run.actor);
    let absent = Action {
        name: STEPS[3].name.into(),
        target: None,
        options: Default::default(),
    };
    assert!(prepared.execution(&absent, &equipment).unwrap().is_none());
    assert!(
        select(&prepared, &equipment, "apply_poison").is_none(),
        "no partial three-of-four plan"
    );
}

#[test]
#[ignore = "native ordered composite control; requires private259 inputs"]
fn literal_composite_empty_plan_is_not_ready() {
    let (mut run, equipment) = native_player(259);
    let prepared = prepare(&mut run, &equipment, "empty_setup", &[]);
    assert!(select(&prepared, &equipment, "empty_setup").is_none());
}

#[test]
#[ignore = "native ordered composite control; requires private259 inputs"]
fn literal_composite_world_execute_rejects_without_dispatching_first_step() {
    let (mut run, _) = native_player(259);
    assert!(!owned_aura(&run, COATINGS[0]));
    let plan = Execution::Composite(vec![
        Execution::Spell(COATINGS[0], run.actor, None),
        Execution::Spell(COATINGS[1], run.actor, None),
    ]);
    let error = execution::execute(run.app.world_mut(), run.actor, plan).unwrap_err();
    assert!(error.contains("precombat"), "{error}");
    assert!(
        run.app
            .world()
            .get::<shared::casting::CastState>(run.actor)
            .is_none()
    );
    assert!(!owned_aura(&run, COATINGS[0]));
}

#[test]
#[ignore = "native ordered composite non-poison control; requires private71 inputs"]
fn literal_composite_native_warrior_buff_is_not_a_poison_special_case() {
    const BATTLE_SHOUT: u32 = 6673;
    let (mut run, equipment) = native_player(71);
    crate::auras::remove_auras(
        run.app.world_mut(),
        run.actor,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == BATTLE_SHOUT,
    );
    assert!(!owned_aura(&run, BATTLE_SHOUT));
    let steps = [NativeAction {
        name: "battle_shout",
        spell: BATTLE_SHOUT,
        talent: None,
        self_target: true,
    }];
    let prepared = prepare(&mut run, &equipment, "native_buff_setup", &steps);
    assert_native_steps_ready(&prepared, &equipment, &steps, run.actor);
    assert_eq!(
        pending(
            select(&prepared, &equipment, "native_buff_setup"),
            run.actor
        ),
        vec![BATTLE_SHOUT]
    );
    assert!(!owned_aura(&run, BATTLE_SHOUT));
}
