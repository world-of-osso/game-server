//! Earn the actual maximum Fury stack through Dragonrage/Animosity, never a test buff.
use super::*;

fn living_flame_hit(run: &mut Run, primary: Entity) -> (i32, bool) {
    let _seed = crate::random::seed_scope(1271796);
    let before = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    crate::spell_cast::creature_cast_with_targets(
        run.app.world_mut(),
        run.actor,
        361469,
        crate::spell_cast::CastTargets {
            unit: primary,
            primary: Some(primary),
            destination: None,
        },
    )
    .unwrap();
    for _ in 0..120 {
        run.app.update();
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        if let Some(event) = events[before..].iter().find(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(primary.to_bits())
                && event.spell_id == Some(361500)
                && event.kind == shared::protocol::CombatLogKind::Damage
        }) {
            let result = (event.amount, event.crit);
            preparation::finish_pending_setup(run).unwrap();
            return result;
        }
    }
    panic!("actual native Living Flame must reach the selected primary");
}

fn disintegrate_tick(run: &mut Run, primary: Entity) -> (i32, bool) {
    let _seed = crate::random::seed_scope(1271796);
    let before = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    crate::spell_cast::creature_cast_with_targets(
        run.app.world_mut(),
        run.actor,
        356995,
        crate::spell_cast::CastTargets {
            unit: primary,
            primary: Some(primary),
            destination: None,
        },
    )
    .unwrap();
    for _ in 0..120 {
        run.app.update();
        if let Some(event) = run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0[before..]
            .iter()
            .find(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.target == Some(primary.to_bits())
                    && event.spell_id == Some(356995)
                    && event.kind == shared::protocol::CombatLogKind::Damage
            })
        {
            return (event.amount, event.crit);
        }
    }
    panic!("actual native Disintegrate must tick on the selected primary");
}

fn fury_sample(selected: bool) -> ((i32, bool), (i32, bool), (i32, bool), f64) {
    // The actual setup/empower sequence rolls equipment procs before the hit.
    // Reset the complete sequence, not only the final cast, for paired inputs.
    let _setup_seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(1467, 1).unwrap();
    engage(&mut run);
    let primary = run.targets[0];
    let percent =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, 1271796, 0);
    assert!(percent > 0.0);
    if !selected {
        crate::auras::remove_auras(
            run.app.world_mut(),
            run.actor,
            crate::auras::RemoveReason::Unlearned,
            |aura| aura.spell_id == 1271796,
        );
    }
    crate::spell_cast::creature_cast_with_targets(
        run.app.world_mut(),
        run.actor,
        375087,
        crate::spell_cast::CastTargets {
            unit: run.actor,
            primary: Some(primary),
            destination: None,
        },
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let before_maximum = living_flame_hit(&mut run, primary);
    for (spell, unit) in [(357208, run.actor), (359073, primary)] {
        crate::spell_cast::creature_cast_empowered_with_targets(
            run.app.world_mut(),
            run.actor,
            spell,
            crate::spell_cast::CastTargets {
                unit,
                primary: Some(primary),
                destination: None,
            },
            1,
        )
        .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
    }
    let fury = |run: &Run| {
        run.app
            .world()
            .get::<crate::auras::ActiveAuras>(run.actor)
            .and_then(|auras| auras.iter().find(|aura| aura.spell_id == 1271783))
            .cloned()
    };
    for _ in 0..1200 {
        if fury(&run).is_some_and(|aura| aura.stacks > 1 && aura.stacks == aura.max_stacks) {
            break;
        }
        run.app.update();
    }
    let aura =
        fury(&run).expect("real Dragonrage and two native Animosity releases must earn Fury cap");
    assert_eq!(aura.stacks, aura.max_stacks);
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 375087),
        "actual Dragonrage must still fix the ordinary mastery condition at maximum Fury"
    );
    let maximum = living_flame_hit(&mut run, primary);
    let periodic = disintegrate_tick(&mut run, primary);
    (
        before_maximum,
        maximum,
        periodic,
        1.0 + f64::from(percent) / 100.0,
    )
}

#[test]
#[ignore = "requires exact Devastation native profile and DB"]
fn evoker_rising_damage_native_second_rank_applies_only_at_actual_fury_cap() {
    let absent = fury_sample(false);
    let selected = fury_sample(true);
    assert_eq!(
        selected.0, absent.0,
        "below Fury cap, rank2 must not add damage"
    );
    assert_eq!(
        selected.1.1, absent.1.1,
        "matched native maximum-stack hits must retain the same crit result"
    );
    // SimC parse_effects.cpp716–719 honors the native EX_SUPPRESS_STACKING flag.
    let expected = f64::from(absent.1.0) * selected.3;
    assert!(
        (f64::from(selected.1.0) - expected).abs() <= 2.0,
        "actual maximum Fury Living Flame must receive one native rank2 bonus: absent={:?}, selected={:?}, factor={}",
        absent.1,
        selected.1,
        selected.3
    );
    assert_eq!(
        selected.2.1, absent.2.1,
        "matched Disintegrate ticks retain their crit result"
    );
    let periodic_expected = f64::from(absent.2.0) * selected.3;
    assert!(
        (f64::from(selected.2.0) - periodic_expected).abs() <= 2.0,
        "actual maximum Fury Disintegrate receives one bonus: absent={:?}, selected={:?}, factor={}",
        absent.2,
        selected.2,
        selected.3
    );
}
