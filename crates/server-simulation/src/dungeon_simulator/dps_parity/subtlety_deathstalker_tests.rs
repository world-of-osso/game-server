//! Native Subtlety Deathstalker mark and its 3-target Black Powder payout.
use super::*;
use shared::{components::PowerType, protocol::CombatLogKind};

fn cast(run: &mut Run, spell: u32) {
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
        literal_rotation::execution::Execution::Spell(spell, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(run).unwrap();
}

#[test]
#[ignore = "requires private native261 profile/traits and player actor"]
fn subtlety_deathstalker_shadowstrike_marks_and_finishers_spend_native_mark() {
    for (finisher, targets) in [(196819, 1), (319175, 3)] {
        let mut run = prepare_literal(261, targets).unwrap();
        engage(&mut run);
        crate::auras::apply_aura(
            run.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: run.actor,
                target: run.actor,
                spell_id: 185313,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
        cast(&mut run, 185438);
        assert!(
            crate::auras::has_aura(run.app.world(), run.targets[0], 457129),
            "real Shadowstrike establishes owned Deathstalker's Mark"
        );
        run.app
            .world_mut()
            .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .clear();
        cast(&mut run, finisher);
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
                    && event.spell_id == Some(457157)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0),
            "actual finisher spends the mark and dispatches native mark damage"
        );
        if targets == 3 {
            for target in &run.targets[1..] {
                assert!(
                    events
                        .iter()
                        .any(|event| event.target == Some(target.to_bits())
                            && event.spell_id == Some(1273061)
                            && event.kind == CombatLogKind::Damage
                            && event.amount > 0),
                    "Black Powder mark payout damages each secondary recipient"
                );
            }
            assert!(
                !events
                    .iter()
                    .any(|event| event.target == Some(run.targets[0].to_bits())
                        && event.spell_id == Some(1273061)
                        && event.kind == CombatLogKind::Damage),
                "Mass Casualty excludes the marked primary"
            );
        }
    }
}
