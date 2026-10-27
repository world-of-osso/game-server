//! SimC76f8118 sc_rogue.cpp6444–6463/9011–9028: a landed finisher
//! distributes Goremaw's residual damage to targets with its live DoT.
use super::*;
use shared::{components::PowerType, protocol::CombatLogKind};

#[test]
#[ignore = "requires private native261 profile/traits and player actor"]
fn subtlety_goremaw_finisher_hits_each_active_bite_target() {
    let mut run = prepare_literal(261, 3).unwrap();
    engage(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(426591, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    for target in &run.targets {
        assert!(
            crate::auras::has_aura(run.app.world(), *target, 426593),
            "real Goremaw's Bite must apply its DoT to admitted targets"
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
    crate::auras::apply_aura(run.app.world_mut(), &crate::auras::AuraApplication {
        caster: run.actor, target: run.actor, spell_id: 1264521,
        effect_mask: u32::MAX, source: crate::auras::AuraSource::Cast,
    });
    run.app
        .world_mut()
        .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .clear();
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
    assert!(events.iter().any(|event| event.spell_id == Some(328082)
        && event.target == Some(run.targets[0].to_bits()) && event.kind == CombatLogKind::Damage
        && event.amount > 0), "real Eviscerate must produce its Shadowed Finishers child");
    for target in &run.targets {
        let hits = events.iter().filter(|event| event.source == Some(run.actor.to_bits())
            && event.target == Some(target.to_bits()) && event.spell_id == Some(1309309)
            && event.kind == CombatLogKind::Damage && event.amount > 0).count();
        assert!(hits >= 2, "the real finisher and its Shadowed Finishers child each emit a residual: {hits}");
    }
}
