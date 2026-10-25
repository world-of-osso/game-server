//! Native boundaries behind the largest measured Fire63 ability gaps.
use super::*;
use crate::auras::ActiveAuras;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use server::simc_apl::Facts;
use shared::components::Health;
use shared::protocol::CombatLogKind;

fn stacks(run: &Run, spell: u32) -> u8 {
    run.app
        .world()
        .get::<ActiveAuras>(run.actor)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == spell && aura.caster == run.actor)
        })
        .map_or(0, |aura| aura.stacks)
}

fn start_native(run: &mut Run, spell: u32, target: Entity) {
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(spell, target, Some(run.targets[0])),
    )
    .unwrap_or_else(|error| panic!("ordinary native{spell}: {error}"));
}

#[test]
#[ignore = "requires exact Fire63 inputs and private70448 DB"]
fn fire_gap_literal_three_targets_casts_selected_flamestrike() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(63, 3).unwrap();
    engage(&mut run);
    advance_combat(&mut run).unwrap();
    assert!(
        run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.spell_id == Some(2120)
                    && event.kind == CombatLogKind::CastSuccess
            }),
        "the selected native2120 must execute in the literal3T fight"
    );
}

#[test]
#[ignore = "requires exact Fire63 inputs and private70448 DB"]
fn fire_gap_sunfury_meteor_guarantees_one_pyroclasm_stack() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_literal(63, 1).unwrap();
    let traits = run
        .app
        .world()
        .get::<crate::class_progression::TraitLoadout>(run.actor)
        .unwrap();
    for talent in [269650, 449349] {
        assert!(
            traits
                .active_talent_spells()
                .any(|spell| spell.spell_id == talent)
        );
    }
    assert_eq!(stacks(&run, 269651), 0);
    let target = run.targets[0];
    start_native(&mut run, 153561, target);
    preparation::finish_pending_setup(&mut run).unwrap();
    assert_eq!(
        stacks(&run, 269651),
        1,
        "Sunfury Meteor bypasses the ordinary15% HS-spend roll"
    );
}

#[test]
#[ignore = "requires exact Fire63 inputs and private70448 DB"]
fn fire_gap_mid2_pyroclasm_hardcasts_crit_after_proc_consumption() {
    let _seed = crate::random::seed_scope(70_448);
    let crits: Vec<_> = (0..5)
        .map(|_| {
            // Separate native actors prevent earlier guaranteed crits from earning
            // Hot Streak and turning a later trial into an instant cast.
            let mut run = prepare_literal(63, 1).unwrap();
            assert!(
                crate::auras::has_aura(run.app.world(), run.actor, 1296583),
                "earned native2pc"
            );
            let target = run.targets[0];
            let first = run.app.world().resource::<ObservedCombatEvents>().0.len();
            // Keep the actual recipient below Firestarter; never change caster crit.
            let mut health = run.app.world_mut().get_mut::<Health>(target).unwrap();
            health.current = health.max * 0.5;
            crate::spell_cast::cast_triggered(
                run.app.world_mut(),
                run.actor,
                269651,
                Some(run.actor),
                1,
            );
            start_native(&mut run, 11366, target);
            assert!(
                run.app
                    .world()
                    .get::<shared::casting::CastState>(run.actor)
                    .is_some(),
                "real hardcast"
            );
            preparation::finish_pending_setup(&mut run).unwrap();
            assert_eq!(
                stacks(&run, 269651),
                0,
                "native proc spent before missile impact"
            );
            for _ in 0..20 {
                run.app.update();
            }
            let hits: Vec<_> = run.app.world().resource::<ObservedCombatEvents>().0[first..]
                .iter()
                .filter(|event| {
                    event.source == Some(run.actor.to_bits())
                        && event.target == Some(target.to_bits())
                        && event.spell_id == Some(11366)
                        && event.kind == CombatLogKind::Damage
                })
                .collect();
            assert_eq!(hits.len(), 1);
            hits[0].crit
        })
        .collect();
    assert!(
        crits.into_iter().all(|crit| crit),
        "native MID2 hardcast guarantee must survive proc consumption"
    );
}

fn reacted(run: &Run, name: &str) -> f64 {
    use crate::simc_apl_native::{Bindings, Encounter, FightStyle, NativeFacts};
    let bindings = Bindings {
        buffs: [("heating_up".into(), 48107), ("hot_streak".into(), 48108)].into(),
        ..default()
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
    .value(name)
    .unwrap()
}

#[test]
#[ignore = "requires exact Fire63 inputs and private70448 DB"]
fn fire_gap_guaranteed_crits_predict_native_hot_streak_reactions() {
    use crate::simc_apl_native::reaction::{ReactionSettings, configure};
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_literal(63, 1).unwrap();
    // Pinned player.cpp1017–1020/1225–1226 reference reaction settings.
    configure(
        run.app.world_mut(),
        run.actor,
        ReactionSettings {
            mean: 0.25,
            stddev: 0.05,
            offset: 0.1,
            nu: 0.15,
            maximum: 1.4,
        },
        &[48107, 48108],
    )
    .unwrap();
    let target = run.targets[0];
    start_native(&mut run, 108853, target);
    run.app.update();
    assert_eq!(
        stacks(&run, 48107),
        1,
        "actual native guaranteed Fire Blast crit"
    );
    assert_eq!(
        reacted(&run, "buff.heating_up.react"),
        1.0,
        "guaranteed crit predicts Heating Up"
    );
    for _ in 0..12 {
        run.app.update();
    }
    start_native(&mut run, 108853, target);
    run.app.update();
    assert_eq!(stacks(&run, 48108), 1);
    assert_eq!(
        reacted(&run, "buff.hot_streak.react"),
        1.0,
        "known HU plus guaranteed crit predicts HS"
    );
    for _ in 0..12 {
        run.app.update();
    }
    start_native(&mut run, 108853, target);
    run.app.update();
    assert_eq!(stacks(&run, 48108), 1, "a wasted crit preserves HS");
    assert_eq!(stacks(&run, 48107), 0, "a wasted crit does not create HU");
    assert_eq!(reacted(&run, "buff.hot_streak.react"), 1.0);
}
