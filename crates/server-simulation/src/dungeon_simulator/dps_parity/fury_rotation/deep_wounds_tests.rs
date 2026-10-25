use super::*;

fn deep_wounds_ticks(run: &Run) -> usize {
    run.app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.kind == shared::protocol::CombatLogKind::Damage
                && event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(262115)
                && event.periodic
        })
        .count()
}

#[test]
#[ignore = "requires exact Fury72 native player inputs"]
fn fury_deep_wounds_rolling_refresh_keeps_next_tick_and_adds_full_native_duration() {
    let _seed_scope = crate::random::seed_scope(70_448);
    let mut run = super::super::prepare_literal(72, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 262115, Some(target), 1);
    for _ in 0..140 {
        run.app.update();
    }
    assert_eq!(
        deep_wounds_ticks(&run),
        6,
        "fresh native6s/1s Deep Wounds pays six ticks"
    );
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 262115, Some(target), 1);
    for _ in 0..10 {
        run.app.update();
    }
    let before = deep_wounds_ticks(&run);
    let schedule = |run: &Run| {
        let aura = run
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(target)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 262115 && aura.caster == actor)
            .unwrap();
        let period = aura
            .effects
            .iter()
            .find(|effect| effect.index == 0)
            .unwrap()
            .period_ms;
        (
            aura.duration_ms,
            period,
            aura.expires_at.unwrap() - crate::auras::aura_now(run.app.world()),
        )
    };
    let initial = schedule(&run);
    assert_eq!((initial.0, initial.1), (6000, 1000));
    let elapsed_ms = f64::from(initial.0) - initial.2 * 1000.0;
    let next_tick_ms = f64::from(initial.1) - elapsed_ms.rem_euclid(f64::from(initial.1));
    let expected_duration = initial.0 + next_tick_ms.round() as u32;
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 262115, Some(target), 1);
    run.app.update();
    assert_eq!(
        schedule(&run).0,
        expected_duration,
        "rolling refresh lifetime includes the pending half-period plus a full native duration"
    );
    for _ in 0..159 {
        run.app.update();
    }
    // SimC76f8118 action.cpp4614–4617/4995–5016: DOT_ROLLING carries
    // time-to-next-full-tick plus a fresh native duration, sharing the old+new budget.
    assert_eq!(
        deep_wounds_ticks(&run) - before,
        7,
        "mid-period rolling refresh retains the pending tick plus six new native ticks"
    );
}

fn cast_native(run: &mut Run, spell: u32, target: Entity) {
    super::super::literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        super::super::literal_rotation::execution::Execution::Spell(
            spell,
            target,
            Some(run.targets[0]),
        ),
    )
    .expect("normal native cast");
    super::super::preparation::finish_pending_setup(run).unwrap();
}

#[test]
#[ignore = "requires exact Arms71 player profile and native selected Deep Wounds"]
fn arms_deep_wounds_still_applies_from_native_mortal_strike() {
    let _seed_scope = crate::random::seed_scope(70_448);
    let mut run = super::super::prepare_literal(71, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    crate::spell_cast::powers::change_power(run.app.world_mut(), actor, PowerType::Rage, 1300.0)
        .unwrap();
    cast_native(&mut run, 12294, target);
    assert!(
        crate::auras::has_aura(run.app.world(), target, 262115),
        "Fury's proc replacement must not disable Arms Mortal Strike's native producer"
    );
}

#[test]
#[ignore = "requires exact Fury72 player profile and native selected Deep Wounds"]
fn fury_deep_wounds_is_applied_by_execute_not_whirlwind() {
    let _seed_scope = crate::random::seed_scope(70_448);
    let mut run = super::super::prepare_measurement(72, 1).unwrap();
    let actor = run.actor;
    cast_native(&mut run, 190411, actor);
    for _ in 0..40 {
        run.app.update();
    }
    let target = run.targets[0];
    assert!(
        !crate::auras::has_aura(run.app.world(), target, 262115),
        "Fury Deep Wounds is an Execute impact relationship, not a Whirlwind proc"
    );
    let maximum = run
        .app
        .world()
        .get::<shared::components::Health>(target)
        .unwrap()
        .max;
    run.app
        .world_mut()
        .get_mut::<shared::components::Health>(target)
        .unwrap()
        .current = maximum * 0.1;
    crate::spell_cast::powers::change_power(run.app.world_mut(), actor, PowerType::Rage, 1300.0)
        .unwrap();
    cast_native(&mut run, 280735, target);
    assert!(
        crate::auras::has_aura(run.app.world(), target, 262115),
        "actual Fury Execute main-hand must retain native Deep Wounds"
    );
}
