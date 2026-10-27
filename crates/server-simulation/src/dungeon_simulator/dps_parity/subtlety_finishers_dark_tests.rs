//! SimC76f8118 sc_rogue.cpp4394–4414/8740–8746: a maximum-point
//! Eviscerate's Shadowed Finishers child snapshots Darkest Night.
use super::*;
use shared::{components::PowerType, protocol::CombatLogKind};

#[test]
#[ignore = "requires private native261 profile/traits and player actor"]
fn subtlety_finishers_dark_child_inherits_guaranteed_critical_chance() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(261, 1).unwrap();
    engage(&mut run);
    run.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .crit_pct = 0.0;
    for id in [1264521, 457280] {
        crate::auras::apply_aura(
            run.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: run.actor,
                target: run.actor,
                spell_id: id,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
    }
    {
        let mut pools = run
            .app
            .world_mut()
            .get_mut::<crate::spell_cast::powers::PowerPools>(run.actor)
            .unwrap();
        for kind in [PowerType::Energy, PowerType::ComboPoints] {
            let pool = pools.get_mut(kind).unwrap();
            pool.current = pool.max;
        }
    }
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(196819, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let child = events
        .iter()
        .find(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(328082)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .expect("real Shadowed Finishers child");
    assert!(
        child.crit,
        "finisher child must retain Darkest Night's guaranteed critical chance"
    );
}
