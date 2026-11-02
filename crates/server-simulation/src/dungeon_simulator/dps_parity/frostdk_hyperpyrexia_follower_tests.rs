//! Two-handed Frost Strike must feed the native Hyperpyrexia residual.
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn cast_when_ready(run: &mut Run, spell: u32) {
    for _ in 0..1200 {
        match crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            spell,
            Some(run.targets[0]),
            false,
        ) {
            Ok(()) => {
                preparation::finish_pending_setup(run).unwrap();
                return;
            }
            Err(
                crate::spell_cast::CastFailure::OnGlobalCooldown
                | crate::spell_cast::CastFailure::OnCooldown
                | crate::spell_cast::CastFailure::NoCharges
                | crate::spell_cast::CastFailure::InProgress
                | crate::spell_cast::CastFailure::NotEnoughPower(_),
            ) => run.app.update(),
            Err(reason) => panic!("Frost cast {spell}: {reason:?}"),
        }
    }
    panic!("Frost cast {spell} did not become ready");
}

#[test]
#[ignore = "native Frost profile and ordinary two-handed casts"]
fn frost_two_hand_strikes_produce_hyperpyrexia() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(251, 1).unwrap();
    assert!(run.app.world().get::<crate::stats::CharacterStats>(run.actor).unwrap().off_hand.is_none());
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 456238));
    engage(&mut run);
    for _ in 0..12 {
        cast_when_ready(&mut run, 49020);
        cast_when_ready(&mut run, 49020);
        cast_when_ready(&mut run, 49143);
    }
    for _ in 0..200 {
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    assert!(events.iter().any(|event| event.spell_id == Some(325464) && event.amount > 0));
    assert!(events.iter().any(|event| event.spell_id == Some(458169)
        && event.kind == CombatLogKind::Damage && event.periodic && event.amount > 0),
        "two-handed Frost Strike must trigger Hyperpyrexia damage");
}
