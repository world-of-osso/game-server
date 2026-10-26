//! SimC76f8118 sc_shaman.cpp6654–6658/7436–7438/13025–13033:
//! ordinary builder executions draw native Routine Communication's shuffled deck.
use super::elemental_mechanics_tests::{cast, fixture};
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

const COMMUNICATION: u32 = 443445;
const SUMMON: u32 = 445624;

fn count_summons(run: &Run) -> usize {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(SUMMON)
                && event.kind == CombatLogKind::CastSuccess
        })
        .count()
}

fn native_deck(run: &Run) -> (usize, usize) {
    let aura = super::elemental_mechanics_tests::own_aura(run, COMMUNICATION)
        .expect("selected native Routine Communication");
    let amount = |index| {
        aura.effects
            .iter()
            .find(|effect| effect.index == index)
            .unwrap()
            .amount as usize
    };
    let positive = amount(4);
    let total = amount(5);
    assert_eq!((positive, total), (4, 50), "native70448 deck inputs");
    (positive, total)
}

fn assert_native_builder_deck(mut run: Run, spell: u32) {
    let (positive, total) = native_deck(&run);
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    // Delayed Overloads and other full-mask background children must not draw.
    for _ in 0..total {
        assert!(crate::spell_cast::try_cast_triggered(
            run.app.world_mut(),
            run.actor,
            spell,
            Some(run.targets[0]),
            1,
        ));
    }
    assert_eq!(count_summons(&run), 0, "background builders never summon");
    for _ in 0..total {
        cast(&mut run, spell);
    }
    assert_eq!(
        count_summons(&run),
        positive,
        "one complete native deck must summon exactly four real Ancestors; Chain recipients and overloads do not add draws"
    );
    let hits = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| event.spell_id == Some(447419) || event.spell_id == Some(447425))
        .filter(|event| event.kind == CombatLogKind::Damage && event.amount > 0)
        .count();
    assert!(
        hits > 0,
        "earned Ancestors must execute native damage, not only summon logs"
    );
}

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_communication_native_bolt_executions_draw_four_of_fifty() {
    let _seed = crate::random::seed_scope(70448);
    assert_native_builder_deck(fixture(), 188196);
}

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_communication_native_chain_draws_once_per_cast_not_recipient() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(262, 3).unwrap();
    engage(&mut run);
    assert_native_builder_deck(run, 188443);
}
