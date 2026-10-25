//! Actual reference profile must execute its precombat stance and combat APL.
//! Master of Warfare is a one-use native replacement, not a permanent filler.
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "private current native DB and exact spec71 inputs"]
fn arms_literal_reference_executes_stance_and_rend_on_three_targets() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(71, 3).unwrap();
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 386164),
        "pinned Arms precombat must actually cast Battle Stance"
    );
    let started = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started);
    engage(&mut run);
    for _ in 0..400 {
        drive_player(&mut run).unwrap();
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    assert!(
        events.iter().any(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(845)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        }),
        "the literal opener must land native Cleave before checking its Rend child"
    );
    let rend_targets: std::collections::BTreeSet<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(388539)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .filter_map(|event| event.target)
        .collect();
    assert_eq!(
        rend_targets.len(),
        3,
        "literal Slayer AoE opens with Cleave to maintain native Rend"
    );
}

#[test]
#[ignore = "private current native DB and exact spec71 inputs"]
fn arms_literal_reference_earns_heroic_without_starving_mortal_strike() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(71, 1).unwrap();
    engage(&mut run);
    for _ in 0..1800 {
        drive_player(&mut run).unwrap();
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    for spell in [12294, 1269383] {
        assert!(
            events.iter().any(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.spell_id == Some(spell)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            }),
            "the native reference must both earn Heroic Strike and keep Mortal Strike: missing{spell}"
        );
    }
}

fn prepare_arms_with_native_rage() -> Run {
    use super::literal_rotation::execution::{Execution, execute};
    let mut run = prepare_measurement(71, 1).unwrap();
    assert_eq!(
        crate::spell_cast::knowledge::resolve_actor_action(run.app.world(), run.actor, 12294),
        Ok(12294),
        "the reference actor must know Mortal Strike before testing replacement starvation"
    );
    engage(&mut run);
    execute(
        run.app.world_mut(),
        run.actor,
        Execution::AutoAttack(run.targets[0]),
    )
    .unwrap();
    for _ in 0..400 {
        run.app.update();
    }
    run.app
        .world_mut()
        .entity_mut(run.actor)
        .remove::<crate::combat::AutoAttack>();
    run
}

#[test]
#[ignore = "private current native DB and exact spec71 inputs"]
fn arms_heroic_strike_requires_its_actual_native_proc() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_arms_with_native_rage();
    assert!(!crate::auras::has_aura(run.app.world(), run.actor, 1269391));
    let spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(1269383)
        .unwrap()
        .clone();
    assert!(
        matches!(
            crate::spell_cast::read_native_cast_start(
                run.app.world_mut(),
                run.actor,
                &spell,
                run.targets[0]
            ),
            Err(crate::spell_cast::CastFailure::CasterAuraState)
        ),
        "Heroic Strike must not replace the rest of the APL without Master of Warfare"
    );
}

#[test]
#[ignore = "private current native DB and exact spec71 inputs"]
fn arms_heroic_strike_consumes_one_native_proc() {
    use super::literal_rotation::execution::{Execution, execute};
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_arms_with_native_rage();
    crate::spell_cast::cast_triggered(run.app.world_mut(), run.actor, 1269391, Some(run.actor), 1);
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 1269391));
    execute(
        run.app.world_mut(),
        run.actor,
        Execution::Spell(1269383, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    run.app.update();
    assert!(
        run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.spell_id == Some(1269383)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
    );
    assert!(
        !crate::auras::has_aura(run.app.world(), run.actor, 1269391),
        "a successful Heroic Strike consumes the old proc without producing another one"
    );
}
