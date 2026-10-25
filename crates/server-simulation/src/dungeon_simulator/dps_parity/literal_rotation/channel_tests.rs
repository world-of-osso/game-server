//! Native channel state and literal source-row cooldown/priority behavior.
use super::*;
use crate::simc_apl_native::{Bindings, FightStyle, NativeFacts};
use execution::NativeAction;
use server::simc_apl::Facts;
use std::collections::BTreeMap;
type Run = super::super::Run;

fn fixture() -> (Run, Equipment, Bindings, Encounter) {
    let mut run = super::super::prepare_literal(62, 1).unwrap();
    // Controller qualification uses the standard native-known-spell fixture.
    // Profile learning/trait legality is separately exercised by spec62 measure.
    let known = run
        .app
        .world()
        .get::<crate::class_progression::KnownSpells>(run.actor)
        .unwrap();
    let spells = known.iter().chain([12051, 5143, 30451]).collect();
    run.app
        .world_mut()
        .entity_mut(run.actor)
        .insert(crate::class_progression::KnownSpells::fixture(spells));
    let mut bindings = Bindings {
        actions: [
            ("evocation".into(), 12051),
            ("arcane_blast".into(), 30451),
            ("arcane_missiles".into(), 5143),
        ]
        .into(),
        resources: [("mana".into(), shared::components::PowerType::Mana)].into(),
        global_cooldown_spell: 30451,
        ..Default::default()
    };
    let equipment = Equipment::load(run.app.world(), run.actor, &mut bindings).unwrap();
    let encounter = Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: FightStyle::Patchwerk,
    };
    (run, equipment, bindings, encounter)
}

const ACTIONS: &[NativeAction<'static>] = &[
    NativeAction {
        name: "arcane_missiles",
        spell: 5143,
        talent: None,
        self_target: false,
    },
    NativeAction {
        name: "evocation",
        spell: 12051,
        talent: None,
        self_target: true,
    },
    NativeAction {
        name: "arcane_blast",
        spell: 30451,
        talent: None,
        self_target: false,
    },
];

#[test]
#[ignore = "requires native62 Player/private profile"]
fn literal_channeling_observes_actual_evocation_not_other_casts() {
    let (mut run, _, bindings, encounter) = fixture();
    let actor = run.actor;
    let channeling = |world: &World| {
        NativeFacts::new(world, actor, &bindings, encounter, 1)
            .value("action.evocation.channeling")
            .unwrap()
    };
    assert_eq!(channeling(run.app.world()), 0.0);
    execution::execute(
        run.app.world_mut(),
        actor,
        Execution::Spell(12051, actor, None),
    )
    .unwrap();
    assert_eq!(channeling(run.app.world()), 1.0);
    crate::spell_cast::cancel_cast(run.app.world_mut(), actor);
    assert_eq!(channeling(run.app.world()), 0.0);
}

#[test]
#[ignore = "requires native62 Player/private profile"]
fn literal_line_cd_is_independent_even_for_identical_source_rows() {
    let (mut run, equipment, _, _) = fixture();
    let mut driver = Driver::new(
        Program::parse(&[
            ("default", "snapshot_stats,line_cd=5"),
            ("default", "snapshot_stats,line_cd=2"),
            ("default", "snapshot_stats,line_cd=5"),
        ])
        .unwrap(),
    );
    let prepared = Prepared::read(
        run.app.world_mut(),
        run.actor,
        &run.targets,
        &equipment,
        [],
        true,
    )
    .unwrap();
    let clock = |time| BTreeMap::from([("time".into(), time)]);
    for row in 0..3 {
        let (action, execution) = driver
            .select("default", &clock(0.0), &prepared, &equipment)
            .unwrap()
            .unwrap();
        assert_eq!(action.source_row, Some(row));
        driver
            .execute(run.app.world_mut(), run.actor, action, execution)
            .unwrap();
    }
    assert!(
        driver
            .select("default", &clock(1.0), &prepared, &equipment)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        driver
            .select("default", &clock(2.0), &prepared, &equipment)
            .unwrap()
            .unwrap()
            .0
            .source_row,
        Some(1)
    );
    assert_eq!(
        driver
            .select("default", &clock(5.0), &prepared, &equipment)
            .unwrap()
            .unwrap()
            .0
            .source_row,
        Some(0)
    );
}

#[test]
#[ignore = "requires native62 Player/private profile"]
fn literal_cancel_action_stops_actual_channel_while_gcd_is_active() {
    let (mut run, equipment, bindings, encounter) = fixture();
    execution::execute(
        run.app.world_mut(),
        run.actor,
        Execution::Spell(12051, run.actor, None),
    )
    .unwrap();
    let mut driver = Driver::new(
        Program::parse(&[(
            "default",
            "cancel_action,if=action.evocation.channeling&mana.pct>=95",
        )])
        .unwrap(),
    );
    let prepared = Prepared::read(
        run.app.world_mut(),
        run.actor,
        &run.targets,
        &equipment,
        ACTIONS.iter().copied(),
        false,
    )
    .unwrap();
    let facts = NativeFacts::new(run.app.world(), run.actor, &bindings, encounter, 1);
    let (action, execution) = driver
        .select("default", &facts, &prepared, &equipment)
        .unwrap()
        .expect("ready cancellation during channel/GCD");
    driver
        .execute(run.app.world_mut(), run.actor, action, execution)
        .unwrap();
    assert!(
        run.app
            .world()
            .get::<shared::casting::CastState>(run.actor)
            .is_none()
    );
}

fn start_driver(
    run: &mut Run,
    equipment: &Equipment,
    bindings: &Bindings,
    encounter: Encounter,
    rows: &[(&str, &str)],
) -> Driver {
    let mut driver = Driver::new(Program::parse(rows).unwrap());
    let prepared = Prepared::read(
        run.app.world_mut(),
        run.actor,
        &run.targets,
        equipment,
        ACTIONS.iter().copied(),
        false,
    )
    .unwrap();
    let facts = NativeFacts::new(run.app.world(), run.actor, bindings, encounter, 1);
    let (action, execution) = driver
        .select("default", &facts, &prepared, equipment)
        .unwrap()
        .unwrap();
    driver
        .execute(run.app.world_mut(), run.actor, action, execution)
        .unwrap();
    assert_eq!(
        run.app
            .world()
            .get::<shared::casting::CastState>(run.actor)
            .unwrap()
            .cast_type,
        shared::casting::CastType::Channel
    );
    driver
}

fn read_decision(
    run: &mut Run,
    driver: &mut Driver,
    equipment: &Equipment,
    bindings: &Bindings,
    encounter: Encounter,
) -> Option<ChannelDecision> {
    let prepared = Prepared::read_after_channel(
        run.app.world_mut(),
        run.actor,
        &run.targets,
        equipment,
        ACTIONS.iter().copied(),
    )
    .unwrap();
    let facts = NativeFacts::new(run.app.world(), run.actor, bindings, encounter, 1);
    driver
        .channel_decision(run.app.world(), run.actor, &facts, &prepared, equipment)
        .unwrap()
}

#[test]
#[ignore = "requires native62 Player/private profile"]
fn literal_interrupt_if_requires_higher_priority_native_readiness() {
    for global in [false, true] {
        let (mut run, equipment, bindings, encounter) = fixture();
        let source = format!(
            "evocation,interrupt_if=1,interrupt_immediate=1,interrupt_global={}",
            u8::from(global)
        );
        let mut driver = start_driver(
            &mut run,
            &equipment,
            &bindings,
            encounter,
            &[
                ("default", "arcane_blast,if=time>=1"),
                ("default", "call_action_list,name=spells"),
                ("spells", &source),
            ],
        );
        let mut interrupted = false;
        for _ in 0..60 {
            run.app.update();
            if let Some(decision) =
                read_decision(&mut run, &mut driver, &equipment, &bindings, encounter)
            {
                assert!(matches!(decision, ChannelDecision::Cancel));
                driver
                    .execute_channel_decision(run.app.world_mut(), run.actor, decision)
                    .unwrap();
                interrupted = true;
                break;
            }
        }
        assert_eq!(
            interrupted, global,
            "only global interruption may see the root's higher priority row"
        );
        if global {
            assert!(
                run.app
                    .world()
                    .get::<shared::casting::CastState>(run.actor)
                    .is_none()
            );
        }
    }
}

#[test]
#[ignore = "requires native62 Player/private profile"]
fn literal_chain_recasts_ready_channel_at_penultimate_native_tick() {
    let (mut run, equipment, bindings, encounter) = fixture();
    // Native263725 is the real admission prerequisite. Two distinct proc inputs
    // isolate channel control; this test makes no claim about their producer.
    let clearcasting = crate::auras::AuraApplication {
        caster: run.actor,
        target: run.actor,
        spell_id: 263725,
        effect_mask: u32::MAX,
        source: crate::auras::AuraSource::Cast,
    };
    crate::auras::apply_aura(run.app.world_mut(), &clearcasting);
    let mut driver = start_driver(
        &mut run,
        &equipment,
        &bindings,
        encounter,
        &[("default", "arcane_missiles,chain=1")],
    );
    crate::auras::apply_aura(run.app.world_mut(), &clearcasting);
    let mut chained = false;
    for _ in 0..200 {
        run.app.update();
        if let Some(decision) =
            read_decision(&mut run, &mut driver, &equipment, &bindings, encounter)
        {
            assert!(matches!(decision, ChannelDecision::Chain(_, _)));
            let cast = run
                .app
                .world()
                .get::<shared::casting::CastState>(run.actor)
                .unwrap();
            assert!(cast.elapsed > 0.0);
            assert!(cast.duration - cast.elapsed <= cast.channel_tick_interval + 0.001);
            driver
                .execute_channel_decision(run.app.world_mut(), run.actor, decision)
                .unwrap();
            assert_eq!(
                run.app
                    .world()
                    .get::<shared::casting::CastState>(run.actor)
                    .unwrap()
                    .elapsed,
                0.0
            );
            assert_eq!(driver.decisions, 2);
            chained = true;
            break;
        }
    }
    assert!(chained, "chain must start a second actual native cast");
}
