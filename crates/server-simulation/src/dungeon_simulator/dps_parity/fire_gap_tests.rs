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

#[test]
#[ignore = "requires exact Fire63 inputs and private70448 DB"]
fn fire_batch_meteor_continues_during_a_new_pyroblast_hardcast() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_literal(63, 1).unwrap();
    let target = run.targets[0];
    start_native(&mut run, 153561, target);
    preparation::finish_pending_setup(&mut run).unwrap();
    start_native(&mut run, 11366, target);
    assert!(
        run.app
            .world()
            .get::<shared::casting::CastState>(run.actor)
            .is_some()
    );
    for _ in 0..140 {
        run.app.update();
    }
    assert!(
        run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.spell_id == Some(153564)
                    && event.kind == CombatLogKind::CastSuccess
            }),
        "an already scheduled Meteor must launch while another spell is casting"
    );
}

#[test]
#[ignore = "requires exact Fire63 inputs and private70448 DB"]
fn fire_batch_fuel_the_fire_rolls_once_for_a_three_target_flamestrike() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_literal(63, 3).unwrap();
    let target = run.targets[0];
    crate::spell_cast::cast_triggered(run.app.world_mut(), run.actor, 190319, Some(run.actor), 1);
    crate::spell_cast::cast_triggered(run.app.world_mut(), run.actor, 48108, Some(run.actor), 1);
    start_native(&mut run, 2120, target);
    preparation::finish_pending_setup(&mut run).unwrap();
    assert_eq!(
        stacks(&run, 48107),
        1,
        "Combustion guarantees the one independent Fuel roll"
    );
    assert_eq!(
        stacks(&run, 48108),
        0,
        "three recipients must not make three Hot Streak rolls"
    );
}

#[test]
#[ignore = "requires exact Fire63 inputs and private70448 DB"]
fn fire_batch_mid2_four_piece_modifies_pyroclasm_payload_not_talent_chance() {
    let mut run = prepare_literal(63, 1).unwrap();
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 1296584));
    crate::spell_cast::cast_triggered(run.app.world_mut(), run.actor, 269651, Some(run.actor), 1);
    let auras = run.app.world().get::<ActiveAuras>(run.actor).unwrap();
    let amount = |spell| {
        auras
            .iter()
            .find(|aura| aura.spell_id == spell)
            .unwrap()
            .effects
            .iter()
            .find(|effect| effect.index == 0)
            .unwrap()
            .amount
    };
    assert_eq!(
        amount(269650),
        15.0,
        "talent label5949 keeps its native15% chance"
    );
    assert_eq!(
        amount(269651),
        255.0,
        "buff label5947 gets230+25 native damage points"
    );
}

#[test]
#[ignore = "requires exact Fire63 inputs and private70448 DB"]
fn fire_batch_mana_cascade_uses_effect_two_haste() {
    let mut run = prepare_literal(63, 1).unwrap();
    let before = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .haste_pct;
    crate::spell_cast::cast_triggered(run.app.world_mut(), run.actor, 449314, Some(run.actor), 1);
    run.app.update();
    let after = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .haste_pct;
    let haste_factor = (100.0 + after) / (100.0 + before);
    assert!(
        (haste_factor - 1.005).abs() < 0.0001,
        "one native stack multiplies haste by1.005: {before} -> {after}"
    );
}

#[test]
#[ignore = "requires exact Fire63 inputs and private70448 DB"]
fn fire_producer_hot_streak_expenditure_grants_mana_cascade() {
    let mut run = prepare_literal(63, 1).unwrap();
    let target = run.targets[0];
    crate::spell_cast::cast_triggered(run.app.world_mut(), run.actor, 48108, Some(run.actor), 1);
    start_native(&mut run, 11366, target);
    assert_eq!(
        stacks(&run, 449314),
        1,
        "selected Mana Cascade gains one stack per Hot Streak spend"
    );
}

fn count_earned_fire_proc(spell: u32) -> usize {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_literal(63, 1).unwrap();
    let target = run.targets[0];
    for _ in 0..64 {
        crate::spell_cast::cast_triggered(
            run.app.world_mut(),
            run.actor,
            48108,
            Some(run.actor),
            1,
        );
        start_native(&mut run, 11366, target);
        preparation::finish_pending_setup(&mut run).unwrap();
        for _ in 0..20 {
            run.app.update();
        }
    }
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(spell)
                && matches!(
                    event.kind,
                    CombatLogKind::AuraApplied | CombatLogKind::AuraRefreshed
                )
        })
        .count()
}

#[test]
#[ignore = "requires exact Fire63 inputs and private70448 DB"]
fn fire_producer_hot_streak_expenditures_earn_spellfire_spheres() {
    assert!(
        count_earned_fire_proc(448604) > 0,
        "selected Spellfire Spheres must proc from Hot Streak expenditures"
    );
}

#[test]
#[ignore = "requires exact Fire63 inputs and private70448 DB"]
fn fire_producer_hot_streak_expenditures_earn_fired_up() {
    assert!(
        count_earned_fire_proc(1257350) > 0,
        "selected Fired Up must proc from Hot Streak expenditures"
    );
}

#[test]
#[ignore = "requires exact Fire63 inputs and private70448 DB"]
fn fire_producer_combustion_expiry_grants_memory_of_alar_hyperthermia() {
    let mut run = prepare_literal(63, 1).unwrap();
    crate::spell_cast::cast_triggered(run.app.world_mut(), run.actor, 190319, Some(run.actor), 1);
    for _ in 0..600 {
        run.app.update();
        if !crate::auras::has_aura(run.app.world(), run.actor, 190319) {
            assert!(
                stacks(&run, 383874) > 0,
                "Memory of Al'ar starts Hyperthermia when Combustion ends"
            );
            return;
        }
    }
    panic!("native Combustion must expire within30s without casts");
}

#[test]
#[ignore = "requires exact Fire63 inputs and private70448 DB"]
fn fire_hero_precombat_clamps_pinned_initial_spellfire_spheres_to_native_cap() {
    let run = prepare_measurement(63, 1).unwrap();
    assert_eq!(
        stacks(&run, 448604),
        3,
        "pinned initial5 applications must fill the native3-stack cap"
    );
}

#[test]
#[ignore = "requires exact Fire63 inputs and private70448 DB"]
fn fire_hero_combustion_consumes_codex_spheres() {
    let mut run = prepare_literal(63, 1).unwrap();
    for _ in 0..5 {
        crate::spell_cast::cast_triggered(
            run.app.world_mut(),
            run.actor,
            448604,
            Some(run.actor),
            1,
        );
    }
    assert_eq!(stacks(&run, 448604), 3, "native Sphere stack cap");
    let actor = run.actor;
    start_native(&mut run, 190319, actor);
    assert_eq!(
        stacks(&run, 448604),
        0,
        "Codex consumes the stored spheres on Phoenix invocation"
    );
}

#[test]
#[ignore = "requires exact Fire63 inputs and private70448 DB"]
fn fire_hero_recent_heating_up_survives_a_noncritical_fireball() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_literal(63, 1).unwrap();
    let target = run.targets[0];
    let mut health = run.app.world_mut().get_mut::<Health>(target).unwrap();
    health.current = health.max * 0.5;
    crate::spell_cast::cast_triggered(run.app.world_mut(), run.actor, 108853, Some(target), 1);
    crate::spell_cast::cast_triggered(run.app.world_mut(), run.actor, 133, Some(target), 1);
    for _ in 0..6 {
        run.app.update();
    }
    let hits: Vec<_> = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(133)
                && event.kind == CombatLogKind::Damage
        })
        .collect();
    assert_eq!(hits.len(), 1);
    assert!(
        !hits[0].crit,
        "native seeded noncritical Fireball precondition"
    );
    assert_eq!(
        stacks(&run, 48107),
        1,
        "noncritical hits cannot erase Heating Up in its first200ms"
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
