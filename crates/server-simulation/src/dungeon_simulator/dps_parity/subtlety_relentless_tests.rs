//! Native Relentless Strikes returns Energy per effective combo point spent.
use super::*;
use shared::{components::PowerType, protocol::CombatLogKind};

#[test]
#[ignore = "requires private native261 profile/traits and player actor"]
fn subtlety_relentless_actual_eviscerate_refunds_native_energy_per_spent_point() {
    for points in [3.0, 7.0] {
        let mut run = prepare_literal(261, 1).unwrap();
        engage(&mut run);
        assert!(
            crate::auras::has_aura(run.app.world(), run.actor, 58423),
            "selected native Relentless Strikes"
        );
        let per_point =
            crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, 98440, 0);
        {
            let mut pools = run
                .app
                .world_mut()
                .get_mut::<crate::spell_cast::powers::PowerPools>(run.actor)
                .unwrap();
            pools.get_mut(PowerType::Energy).unwrap().current = 80.0;
            pools.get_mut(PowerType::ComboPoints).unwrap().current = points;
        }
        literal_rotation::execution::execute(
            run.app.world_mut(),
            run.actor,
            literal_rotation::execution::Execution::Spell(
                196819,
                run.targets[0],
                Some(run.targets[0]),
            ),
        )
        .unwrap();
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        assert!(
            events
                .iter()
                .any(|event| event.source == Some(run.actor.to_bits())
                    && event.target == Some(run.targets[0].to_bits())
                    && event.spell_id == Some(196819)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0),
            "actual native finisher hits"
        );
        let refunds: Vec<_> = events
            .iter()
            .filter(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.target == Some(run.actor.to_bits())
                    && event.spell_id == Some(98440)
                    && event.kind == CombatLogKind::Energize
            })
            .collect();
        assert_eq!(refunds.len(), 1, "one native refund per landed finisher");
        assert_eq!(
            refunds[0].amount,
            (per_point * points) as i32,
            "native value times effective points"
        );
        assert_eq!(
            refunds[0].overflow, 0,
            "fixture leaves room for full refund"
        );
    }
}
