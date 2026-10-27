//! Mass Casualty uses the already-owned mark, not an unrelated secondary target.
use super::*;
use shared::{components::PowerType, protocol::CombatLogKind};

#[test]
#[ignore = "requires private native261 profile/traits and player actor"]
fn subtlety_mass_black_powder_spends_mark_and_hits_only_secondaries() {
    let mut run = prepare_literal(261, 3).unwrap();
    engage(&mut run);
    crate::auras::apply_aura(
        run.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: run.actor,
            target: run.targets[0],
            spell_id: 457129,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    assert!(crate::auras::set_owned_stacks(
        run.app.world_mut(),
        run.actor,
        run.targets[0],
        457129,
        3
    ));
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
        literal_rotation::execution::Execution::Spell(319175, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    for (index, target) in run.targets.iter().enumerate() {
        let hits = events
            .iter()
            .filter(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(1273061)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
            .count();
        assert_eq!(
            hits,
            usize::from(index != 0),
            "native Mass Casualty targets only secondary victims"
        );
    }
}
