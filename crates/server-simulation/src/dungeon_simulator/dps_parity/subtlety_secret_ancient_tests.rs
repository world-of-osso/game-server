//! SimC76f8118 sc_rogue.cpp5435–5503/8668–8701: Secret Technique's
//! earned Ancient Arts finisher refills points and launches its own clone.
use super::*;
use shared::{components::PowerType, protocol::CombatLogKind};

#[test]
#[ignore = "requires private native261 profile/traits and player actor"]
fn subtlety_secret_ancient_actual_finisher_collects_and_launches_clone() {
    let mut run = prepare_literal(261, 3).unwrap();
    engage(&mut run);
    for id in [196911, 1269163] {
        crate::auras::apply_aura(run.app.world_mut(), &crate::auras::AuraApplication {
            caster: run.actor, target: run.actor, spell_id: id,
            effect_mask: u32::MAX, source: crate::auras::AuraSource::Cast,
        });
    }
    assert!(crate::auras::set_self_cast_stacks(run.app.world_mut(), run.actor, 196911, 7));
    {
        let mut pools = run.app.world_mut().get_mut::<crate::spell_cast::powers::PowerPools>(run.actor).unwrap();
        for kind in [PowerType::Energy, PowerType::ComboPoints] {
            let pool = pools.get_mut(kind).unwrap();
            pool.current = pool.max;
        }
    }
    literal_rotation::execution::execute(run.app.world_mut(), run.actor,
        literal_rotation::execution::Execution::Spell(280719, run.targets[0], Some(run.targets[0]))).unwrap();
    let (points, _) = crate::spell_cast::powers::power_value(run.app.world(), run.actor, PowerType::ComboPoints).unwrap();
    assert_eq!(points, 7.0, "actual Secret Technique must collect its earned stored points after payment");
    preparation::finish_pending_setup(&mut run).unwrap();
    let events = &run.app.world().resource::<crate::spell_cast::test_harness::ObservedCombatEvents>().0;
    for target in &run.targets {
        assert!(events.iter().any(|event| event.source == Some(run.actor.to_bits()) && event.target == Some(target.to_bits())
            && event.spell_id == Some(1269560) && event.kind == CombatLogKind::Damage && event.amount > 0),
            "earned Secret Technique shadow clone must reach each target");
    }
}
