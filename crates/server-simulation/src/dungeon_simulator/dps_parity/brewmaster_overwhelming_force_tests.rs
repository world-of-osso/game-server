//! Real primary strikes produce an unmodified Overwhelming Force copy.
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires canonical Brewmaster profile and world database"]
fn primary_monk_strikes_copy_their_resolved_damage() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(268, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    let percent =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), actor, 451024, 0) / 100.0;
    for spell in [205523, 100780] {
        let start = run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .len();
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, spell, Some(target), false)
            .unwrap();
        for _ in 0..150 {
            run.app.update();
        }
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0[start..];
        let source = events
            .iter()
            .find(|event| {
                event.spell_id == Some(spell)
                    && event.kind == CombatLogKind::Damage
                    && event.target == Some(target.to_bits())
            })
            .expect("primary strike damage");
        let copies: Vec<_> = events
            .iter()
            .filter(|event| {
                event.spell_id == Some(452333)
                    && event.kind == CombatLogKind::Damage
                    && event.target == Some(target.to_bits())
            })
            .collect();
        assert_eq!(copies.len(), 1, "one copy per primary strike");
        let expected = source.amount as f32 * percent;
        assert!(
            (copies[0].amount as f32 - expected).abs() <= 2.0,
            "copy must not reroll crit or reapply armor: {} vs {expected}",
            copies[0].amount
        );
    }
}
