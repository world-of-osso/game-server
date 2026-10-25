//! Actual native Player70 reset producers, not injected proc events or fabricated spell rows.
use super::*;
use crate::spell_cast::{CastFailure, test_harness::ObservedCombatEvents};
use shared::protocol::CombatLogKind;

fn player_in_melee() -> Run {
    let (profile, traits) = read_reference(70).unwrap();
    let mut run = prepare_reference(
        profile,
        &traits,
        1,
        crate::followers::simulator::ParityActor::Player { race: 3 },
    )
    .unwrap();
    engage(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::AutoAttack(run.targets[0]),
    )
    .unwrap();
    run
}

fn applications(run: &Run, spell: u32) -> usize {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(spell)
                && event.kind == CombatLogKind::AuraApplied
        })
        .count()
}

#[test]
#[ignore = "native70448 Player70 autoattack proc qualification"]
fn ret_auto_proc_art_of_war_earns_buff_and_resets_blade() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = player_in_melee();
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 406064));
    for _ in 0..6000 {
        let spell = run
            .app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(184575)
            .unwrap()
            .clone();
        let before = crate::spell_cast::read_native_cast_after_channel(
            run.app.world_mut(),
            run.actor,
            &spell,
            run.targets[0],
        );
        let old_applications = applications(&run, 406086);
        if before.is_ok() {
            match crate::spell_cast::cast_for_player_with_target(
                run.app.world_mut(),
                run.actor,
                184575,
                Some(run.targets[0]),
            ) {
                Ok(())
                | Err(
                    CastFailure::OnGlobalCooldown
                    | CastFailure::NoCharges
                    | CastFailure::InProgress,
                ) => {}
                Err(reason) => panic!("native Blade failed: {reason:?}"),
            }
        }
        run.app.update();
        if applications(&run, 406086) > old_applications && before == Err(CastFailure::NoCharges) {
            assert!(
                crate::spell_cast::read_native_cast_after_channel(
                    run.app.world_mut(),
                    run.actor,
                    &spell,
                    run.targets[0]
                )
                .is_ok(),
                "earned Art of War restores an exhausted Blade charge"
            );
            return;
        }
    }
    panic!("native autoattacks did not earn Art of War while Blade charges were exhausted");
}

#[test]
#[ignore = "native70448 earned Art of War stacks must be consumed by ordinary Blade"]
fn ret_auto_proc_blade_consumes_one_earned_art_of_war_stack() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = player_in_melee();
    let stacks = |run: &Run| {
        run.app
            .world()
            .get::<crate::auras::ActiveAuras>(run.actor)
            .and_then(|auras| auras.iter().find(|aura| aura.spell_id == 406086))
            .map_or(0, |aura| aura.stacks)
    };
    for _ in 0..6000 {
        run.app.update();
        if stacks(&run) == 2 {
            crate::spell_cast::cast_for_player_with_target(
                run.app.world_mut(),
                run.actor,
                184575,
                Some(run.targets[0]),
            )
            .unwrap();
            assert_eq!(
                stacks(&run),
                1,
                "selected Light Within consumes one earned Art of War stack on Blade"
            );
            return;
        }
    }
    panic!("actual repeating attacks must earn two native Art of War stacks");
}

#[test]
#[ignore = "native70448 Walk Into Light grants its AW resource and buff payload"]
fn ret_auto_proc_wrath_grants_walk_into_light_holy_power_and_anshe() {
    let mut run = player_in_melee();
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 1263782));
    assert_eq!(ret_purpose_tests::current_holy_power(&run), 0.0);
    crate::spell_cast::cast_for_player_with_target(
        run.app.world_mut(),
        run.actor,
        31884,
        Some(run.actor),
    )
    .unwrap();
    assert_eq!(
        ret_purpose_tests::current_holy_power(&run),
        2.0,
        "ordinary Wrath must grant native Walk Into Light effect2 Holy Power"
    );
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 445206),
        "the same successful roll grants the native Ret Blessing of Anshe"
    );
}

fn admitted_without_gcd(run: &mut Run, spell: u32) -> Result<Option<Entity>, CastFailure> {
    let spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(spell)
        .unwrap()
        .clone();
    crate::spell_cast::read_native_cast_after_channel(
        run.app.world_mut(),
        run.actor,
        &spell,
        run.targets[0],
    )
}

fn attempt_ordinary_cast(run: &mut Run, spell: u32, target: Entity) -> Result<(), CastFailure> {
    crate::spell_cast::cast_for_player_with_target(
        run.app.world_mut(),
        run.actor,
        spell,
        Some(target),
    )
}

#[test]
#[ignore = "native70448 Final Verdict must roll its authored two-action reset"]
fn ret_auto_proc_final_verdict_restores_judgment_and_hammer_charges() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = player_in_melee();
    let mut exhausted_verdicts = 0;
    for _ in 0..12000 {
        let actor = run.actor;
        let target = run.targets[0];
        let _ = attempt_ordinary_cast(&mut run, 31884, actor);
        for spell in [20271, 24275] {
            let _ = attempt_ordinary_cast(&mut run, spell, target);
        }
        let judgment_exhausted =
            admitted_without_gcd(&mut run, 20271) == Err(CastFailure::NoCharges);
        let hammer_exhausted = admitted_without_gcd(&mut run, 24275) == Err(CastFailure::NoCharges);
        if judgment_exhausted
            && hammer_exhausted
            && attempt_ordinary_cast(&mut run, 383328, target).is_ok()
        {
            exhausted_verdicts += 1;
            let judgment_ready = admitted_without_gcd(&mut run, 20271).is_ok();
            let hammer_ready = admitted_without_gcd(&mut run, 24275).is_ok();
            if judgment_ready || hammer_ready {
                assert!(
                    judgment_ready && hammer_ready,
                    "one Final Verdict roll must restore both authored actions"
                );
                return;
            }
        }
        run.app.update();
    }
    assert!(
        exhausted_verdicts > 0,
        "ordinary paid Verdict must exercise the exhausted native pools"
    );
    panic!("{exhausted_verdicts} paid Final Verdicts never restored Judgment and Hammer charges");
}
