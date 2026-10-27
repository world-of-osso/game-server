//! SimC76f8118 sc_rogue.cpp5405–5413: actual Secret Technique
//! clones can earn Shadow Techniques storage and Energy.
use super::*;
use shared::{components::PowerType, protocol::CombatLogKind};

#[test]
#[ignore = "requires private native261 profile/traits and player actor"]
fn subtlety_secret_funding_delayed_clones_can_store_and_energize() {
    let _seed = crate::random::seed_scope(70448);
    let mut funded = false;
    for _ in 0..10 {
        let mut run = prepare_literal(261, 1).unwrap();
        engage(&mut run);
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
            literal_rotation::execution::Execution::Spell(
                280719,
                run.targets[0],
                Some(run.targets[0]),
            ),
        )
        .unwrap();
        assert!(
            !crate::auras::has_aura(run.app.world(), run.actor, 196911),
            "no pre-existing storage before delayed clones"
        );
        run.app
            .world_mut()
            .entity_mut(run.actor)
            .insert(crate::combat::AutoAttack {
                victim: run.targets[0],
                timer: std::time::Duration::from_secs(60),
                off_hand_timer: std::time::Duration::from_secs(60),
            });
        run.app
            .world_mut()
            .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .clear();
        preparation::finish_pending_setup(&mut run).unwrap();
        for _ in 0..100 {
            run.app.update();
        }
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        assert!(
            !events
                .iter()
                .any(|event| event.source == Some(run.actor.to_bits())
                    && event.spell_id.is_none()
                    && event.kind == CombatLogKind::Damage),
            "white swings cannot fund the test"
        );
        assert_eq!(
            events
                .iter()
                .filter(
                    |event| event.spell_id == Some(282449) && event.kind == CombatLogKind::Damage
                )
                .count(),
            2,
            "both real delayed Secret Technique clones must land"
        );
        funded |= crate::auras::has_aura(run.app.world(), run.actor, 196911)
            && events.iter().any(|event| {
                event.spell_id == Some(196911)
                    && event.kind == CombatLogKind::Energize
                    && event.amount > 0
            });
        if funded {
            break;
        }
    }
    assert!(
        funded,
        "seeded Secret Technique clones must earn native storage and Energy"
    );
}
