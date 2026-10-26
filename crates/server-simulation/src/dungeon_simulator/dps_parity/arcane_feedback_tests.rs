//! Real Barrage outcomes own the selected native Overflowing Energy clock.
use super::arcane_tests::start_cast;
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

const OVERFLOWING: u32 = 394195;
const BARRAGE: u32 = 44425;

fn overflowing_stacks(run: &Run) -> u8 {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.caster == run.actor && aura.spell_id == OVERFLOWING)
        .map_or(0, |aura| aura.stacks)
}

fn cast_and_observe_critical(run: &mut Run, spell: u32) -> bool {
    let first = run.app.world().resource::<ObservedCombatEvents>().0.len();
    start_cast(run, spell);
    preparation::finish_pending_setup(run).unwrap();
    for _ in 0..20 {
        run.app.update();
    }
    let hits = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0[first..]
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(spell)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .collect::<Vec<_>>();
    assert_eq!(hits.len(), 1, "one actual native{spell} primary impact");
    hits[0].crit
}

#[test]
#[ignore = "requires private native70448 DB and exact Arcane62 inputs"]
fn arcane_feedback_native_barrage_earns_and_clears_overflowing_energy() {
    // SimC76f8118 sc_mage.cpp2693/1897–1898: Barrage alone is Arcane's
    // eligible producer; merge noncritical gains and critical expiration after
    // the real hit batch. Preserve native crit rolls, gear and aura values.
    let mut run = prepare_literal(62, 1).unwrap();
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 390218));
    let cap = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(OVERFLOWING)
        .unwrap()
        .aura_options
        .as_ref()
        .unwrap()
        .cumulative_aura as u8;
    assert!(cap > 0);
    let _seed = crate::random::seed_scope(70448);
    let mut saw_gain = false;
    let mut saw_clear = false;
    for _ in 0..16 {
        let before = overflowing_stacks(&run);
        let critical = cast_and_observe_critical(&mut run, BARRAGE);
        let expected = if critical { 0 } else { (before + 1).min(cap) };
        assert_eq!(
            overflowing_stacks(&run),
            expected,
            "actual native Barrage critical={critical}, previous stacks={before}"
        );
        if critical {
            saw_clear |= before > 0;
        } else {
            saw_gain = true;
            // sc_mage.cpp2693/2854–2866: Blast is not eligible, even with
            // the same selected class talent and an earned active buff.
            cast_and_observe_critical(&mut run, 30451);
            assert_eq!(overflowing_stacks(&run), expected, "Blast must preserve the earned buff");
        }
        if saw_gain && saw_clear {
            return;
        }
    }
    panic!("bounded native casts must observe both a gain and a critical clear");
}
