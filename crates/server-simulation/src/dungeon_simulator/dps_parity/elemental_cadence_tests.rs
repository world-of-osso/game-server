//! Literal cadence, real background callbacks and native counted spell uses.
use super::elemental_mechanics_tests::{cast, fixture, own_aura, settle};
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn cast_self(run: &mut Run, spell: u32) {
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        spell,
        Some(run.actor),
        false,
    )
    .unwrap();
    settle(run);
}

fn clear_events(run: &mut Run) {
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
}

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_literal_maintains_flame_shock_through_its_native_cast_wrapper() {
    let mut run = prepare_measurement(262, 1).unwrap();
    engage(&mut run);
    clear_events(&mut run);
    let wrapper = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(470411)
        .unwrap()
        .clone();
    let initial_readiness = crate::spell_cast::read_native_cast_start(
        run.app.world_mut(),
        run.actor,
        &wrapper,
        run.targets[0],
    );
    let resolved = crate::spell_cast::knowledge::resolve_action(run.app.world(), run.actor, 470411);
    for _ in 0..1200 {
        drive_player(&mut run).unwrap();
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let casts = events
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.kind == CombatLogKind::CastSuccess
                && event.spell_id == Some(470411)
        })
        .count();
    let ticks = events
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id == Some(188389)
                && event.periodic
        })
        .count();
    let known = run
        .app
        .world()
        .get::<crate::class_progression::KnownSpells>(run.actor)
        .unwrap();
    assert!(
        casts >= 2 && ticks >= 30,
        "native470411→188389 wrapper casts{casts}, ticks{ticks}, initial readiness{initial_readiness:?}, resolved{resolved:?}, known wrapper{}, known payload{}",
        known.contains(470411),
        known.contains(188389)
    );
}

fn fund_native_blast(run: &mut Run) {
    for _ in 0..64 {
        let (current, maximum) = crate::spell_cast::powers::power_value(
            run.app.world(),
            run.actor,
            shared::components::PowerType::Maelstrom,
        )
        .unwrap();
        if current >= maximum {
            return;
        }
        cast(run, 188196);
    }
    panic!("actual Lightning casts did not fill native Maelstrom");
}

fn finish_foreground_cast(run: &mut Run, spell: u32) {
    for _ in 0..300 {
        run.app.update();
        if run
            .app
            .world()
            .get::<shared::casting::CastState>(run.actor)
            .is_none_or(|state| state.spell_id != spell)
        {
            return;
        }
    }
    panic!("native spell{spell} did not complete");
}

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_overload_is_instant_background_while_another_native_cast_is_active() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = fixture();
    fund_native_blast(&mut run);
    cast_self(&mut run, 1219480); // Native effect8 guarantees an extra overload.
    clear_events(&mut run);
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        117014,
        Some(run.targets[0]),
        false,
    )
    .unwrap();
    finish_foreground_cast(&mut run, 117014);
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        188196,
        Some(run.targets[0]),
        false,
    )
    .unwrap();
    assert_eq!(
        run.app
            .world()
            .get::<shared::casting::CastState>(run.actor)
            .unwrap()
            .spell_id,
        188196
    );
    for _ in 0..12 {
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(120588)
                && event.kind == CombatLogKind::Damage),
        "guaranteed native EB overload must hit during the next foreground hardcast"
    );
    assert!(
        !events
            .iter()
            .any(|event| event.spell_id == Some(120588) && event.kind == CombatLogKind::CastStart),
        "background overload must not open a hardcast bar"
    );
    assert_eq!(
        run.app
            .world()
            .get::<shared::casting::CastState>(run.actor)
            .unwrap()
            .spell_id,
        188196,
        "background callback preserves the real foreground cast"
    );
}

fn bolt_prediction(run: &Run) -> f64 {
    use server::simc_apl::Facts;
    let bindings = crate::simc_apl_native::Bindings {
        actions: [("lightning_bolt".into(), 188196)].into(),
        ..Default::default()
    };
    crate::simc_apl_native::NativeFacts::new(
        run.app.world(),
        run.actor,
        &bindings,
        crate::simc_apl_native::Encounter::Fixture {
            started_at: crate::spell_cast::now(run.app.world()),
            duration: 300.0,
            style: crate::simc_apl_native::FightStyle::Patchwerk,
        },
        1,
    )
    .with_targets(run.targets.first().copied(), &run.targets)
    .value("action.lightning_bolt.damage")
    .unwrap()
}

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_stormkeeper_native_uses_do_not_multiply_damage_or_get_spent_by_overloads() {
    let mut run = fixture();
    assert!(
        bolt_prediction(&run) > 0.0,
        "native bolt prediction is available before SK"
    );
    cast_self(&mut run, 191634);
    let keeper = own_aura(&run, 191634).unwrap();
    let uses = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(191634)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 4)
        .unwrap()
        .base_points as u8;
    assert_eq!(
        keeper.stacks, uses,
        "native effect4 supplies the initial next-spell uses"
    );
    let percent = keeper
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .unwrap()
        .amount;
    let instance = keeper.instance_id;
    crate::auras::remove_instance(
        run.app.world_mut(),
        run.actor,
        instance,
        crate::auras::RemoveReason::Consumed,
    );
    let ordinary = bolt_prediction(&run);
    crate::spell_cast::cast_triggered(run.app.world_mut(), run.actor, 191634, Some(run.actor), 1);
    let empowered = bolt_prediction(&run);
    let expected = ordinary * (1.0 + f64::from(percent) / 100.0);
    assert!(
        (empowered - expected).abs() <= 3.0,
        "one native{percent}% bonus, not multiplied by{uses} uses: {ordinary}→{empowered}, expected{expected}"
    );
    for remaining in (0..uses).rev() {
        crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            188196,
            Some(run.targets[0]),
            false,
        )
        .unwrap();
        assert!(
            run.app
                .world()
                .get::<shared::casting::CastState>(run.actor)
                .is_none(),
            "each earned SK use makes its root instant"
        );
        settle(&mut run);
        assert_eq!(
            own_aura(&run, 191634).map_or(0, |aura| aura.stacks),
            remaining,
            "ordinary root spends one; its delayed overload does not spend another"
        );
    }
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        188196,
        Some(run.targets[0]),
        false,
    )
    .unwrap();
    assert!(
        run.app
            .world()
            .get::<shared::casting::CastState>(run.actor)
            .is_some(),
        "no instant cast after the native uses are exhausted"
    );
}
