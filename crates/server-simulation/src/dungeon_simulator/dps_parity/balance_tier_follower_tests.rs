use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires native Balance102 WCL profile and frozen world data"]
fn balance_mid2_starfall_instant_hits_engaged_enemies_only_with_two_piece() {
    use shared::components::PowerType;
    for equipped in [false, true] {
        let _seed = crate::random::seed_scope(70448);
        let mut run = prepare_literal(102, 3).unwrap();
        let untouched = run.targets.pop().unwrap();
        engage(&mut run);
        assert!(crate::auras::has_aura(run.app.world(), run.actor, 1296603));
        if !equipped {
            crate::auras::remove_auras(
                run.app.world_mut(),
                run.actor,
                crate::auras::RemoveReason::Cancelled,
                |aura| aura.spell_id == 1296603,
            );
        }
        let (_, maximum) = crate::spell_cast::powers::power_value(
            run.app.world(),
            run.actor,
            PowerType::LunarPower,
        )
        .unwrap();
        crate::spell_cast::powers::change_power(
            run.app.world_mut(),
            run.actor,
            PowerType::LunarPower,
            maximum,
        );
        let cursor = run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .len();
        crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            191034,
            Some(run.actor),
            false,
        )
        .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
        for _ in 0..10 {
            run.app.update();
        }
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0[cursor..];
        let hits: Vec<_> = events
            .iter()
            .filter(|event| {
                event.kind == CombatLogKind::Damage
                    && event.source == Some(run.actor.to_bits())
                    && event.spell_id == Some(1301742)
            })
            .collect();
        assert_eq!(hits.len(), if equipped { 2 } else { 0 }, "{events:?}");
        for hit in hits {
            assert!(hit.amount > 0);
            assert_ne!(hit.target, Some(untouched.to_bits()));
            assert!(
                run.targets
                    .iter()
                    .any(|target| hit.target == Some(target.to_bits()))
            );
        }
    }
}
