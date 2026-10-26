//! SimC76f8118 sc_rogue.cpp5331–5501: Secret Technique dispatches
//! the actual player attack and two delayed native clone attacks.
use super::*;
use shared::{components::PowerType, protocol::CombatLogKind};

#[test]
#[ignore = "requires private native261 profile/traits and player actor"]
fn subtlety_secret_native_finisher_dispatches_player_and_delayed_clones() {
    let mut run = prepare_literal(261, 3).unwrap();
    engage(&mut run);
    {
        let mut pools = run
            .app
            .world_mut()
            .get_mut::<crate::spell_cast::powers::PowerPools>(run.actor)
            .unwrap();
        let energy = pools.get_mut(PowerType::Energy).unwrap();
        energy.current = energy.max;
        let combo = pools.get_mut(PowerType::ComboPoints).unwrap();
        combo.current = combo.max;
    }
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(280719, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    for _ in 0..100 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    for target in &run.targets {
        let hits = |spell| {
            events
                .iter()
                .filter(|event| {
                    event.source == Some(run.actor.to_bits())
                        && event.target == Some(target.to_bits())
                        && event.spell_id == Some(spell)
                        && event.kind == CombatLogKind::Damage
                        && event.amount > 0
                })
                .count()
        };
        assert_eq!(
            hits(280720),
            1,
            "one native player attack hits each recipient"
        );
        assert_eq!(
            hits(282449),
            2,
            "both delayed native clones hit each recipient"
        );
    }
}
