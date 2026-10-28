//! Real Deathmark grants Finish the Job's native damage window.
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn lingering_damage(retain: bool) -> (i32, f32) {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(259, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    assert!(crate::auras::has_aura(run.app.world(), actor, 457056));
    assert!(!crate::auras::has_aura(run.app.world(), actor, 457273));
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 360194, Some(target), 1);
    assert!(crate::auras::has_aura(run.app.world(), target, 360194));
    assert!(!crate::auras::has_aura(run.app.world(), actor, 457273));
    for _ in 0..400 {
        run.app.update();
    }
    assert!(!crate::auras::has_aura(run.app.world(), target, 360194));
    let aura = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 457273 && aura.caster == actor)
        .expect("Deathmark expiry grants Lingering Darkness");
    let percent = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount;
    if !retain {
        crate::auras::remove_auras(
            run.app.world_mut(),
            actor,
            crate::auras::RemoveReason::Cancelled,
            |aura| aura.spell_id == 457273 && aura.caster == actor,
        );
    }
    run.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(actor)
        .unwrap()
        .crit_pct = 0.0;
    let start = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    let _impact_seed = crate::random::seed_scope(259);
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 113780, Some(target), 1);
    (next_damage(&mut run, Some(113780), false, start), percent)
}

#[test]
#[ignore = "requires prepared259 native profile and world data"]
fn assassination_deathmark_expiry_grants_lingering_damage_once() {
    let (base, _) = lingering_damage(false);
    let (buffed, percent) = lingering_damage(true);
    let expected = base as f32 * (1.0 + percent / 100.0);
    assert!(
        (buffed as f32 - expected).abs() <= 3.0,
        "real poison receives one native Lingering bonus: {base} -> {buffed}, expected {expected}"
    );
}

fn next_damage(run: &mut Run, spell: Option<u32>, periodic: bool, start: usize) -> i32 {
    for _ in 0..160 {
        run.app.update();
        if let Some(event) = run.app.world().resource::<ObservedCombatEvents>().0[start..]
            .iter()
            .find(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.target == Some(run.targets[0].to_bits())
                    && event.spell_id == spell
                    && event.kind == CombatLogKind::Damage
                    && event.periodic == periodic
                    && !event.crit
            })
        {
            return event.amount;
        }
    }
    panic!("actual attack {spell:?} did not deal noncritical damage");
}

fn sample(selected: bool) -> [i32; 3] {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(259, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    assert!(crate::auras::has_aura(run.app.world(), actor, 1249809));
    if !selected {
        crate::auras::remove_auras(
            run.app.world_mut(),
            actor,
            crate::auras::RemoveReason::Cancelled,
            |aura| aura.spell_id == 1249809,
        );
    }
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        actor,
        360194,
        Some(target),
        1
    ));
    run.app.update();
    assert_eq!(
        crate::auras::has_aura(run.app.world(), actor, 1249810),
        selected
    );
    run.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(actor)
        .unwrap()
        .crit_pct = 0.0;
    let start = run.app.world().resource::<ObservedCombatEvents>().0.len();
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        actor,
        5374,
        Some(target),
        1
    ));
    let direct = next_damage(&mut run, Some(5374), false, start);
    let start = run.app.world().resource::<ObservedCombatEvents>().0.len();
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        actor,
        385627,
        Some(target),
        1
    ));
    let periodic = next_damage(&mut run, Some(385627), true, start);
    let start = run.app.world().resource::<ObservedCombatEvents>().0.len();
    crate::combat::start_attack_now(
        run.app.world_mut(),
        actor,
        crate::combat::AutoAttack::new(target),
    );
    let white = next_damage(&mut run, None, false, start);
    [direct, periodic, white]
}

#[test]
#[ignore = "requires native259 profile and Finish the Job"]
fn assassination_deathmark_earns_finish_job_damage_window() {
    let baseline = sample(false);
    let selected = sample(true);
    for (plain, boosted) in baseline.into_iter().zip(selected) {
        assert!(
            (f64::from(boosted) / f64::from(plain) - 1.10).abs() < 0.02,
            "real Deathmark must grant native Finish the Job damage: {plain} -> {boosted}"
        );
    }
}
