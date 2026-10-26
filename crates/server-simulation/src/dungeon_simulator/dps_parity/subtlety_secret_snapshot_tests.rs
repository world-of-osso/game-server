//! Secret Technique clone attacks retain launch crit and their sourced delay.
use super::*;
use shared::{components::PowerType, protocol::CombatLogKind};

#[test]
#[ignore = "requires private native261 profile/traits and player actor"]
fn subtlety_secret_clones_snapshot_critical_chance_before_delayed_impacts() {
    let mut run = prepare_literal(261, 1).unwrap();
    engage(&mut run);
    {
        let mut stats = run
            .app
            .world_mut()
            .get_mut::<crate::stats::CharacterStats>(run.actor)
            .unwrap();
        stats.crit_pct = 100.0;
    }
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
    run.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .crit_pct = 0.0;
    for _ in 0..6 {
        run.app.update();
    }
    let initial = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        !initial
            .iter()
            .any(|event| event.spell_id == Some(282449) && event.kind == CombatLogKind::Damage),
        "normal Secret Technique clones do not hit in the first 300ms"
    );
    for _ in 0..70 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let clones: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(282449)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .collect();
    assert_eq!(clones.len(), 2, "two delayed clone attacks land");
    assert!(
        clones.iter().all(|event| event.crit),
        "both clones retain guaranteed launch-time crit after live crit drops"
    );
}
