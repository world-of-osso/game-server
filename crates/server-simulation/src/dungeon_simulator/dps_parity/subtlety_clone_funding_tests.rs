//! SimC76f8118 sc_rogue.cpp3650–3659/8159–8172: eligible shadow
//! clones can earn Shadow Techniques storage and its native Energy.
use super::*;
use shared::{components::PowerType, protocol::CombatLogKind};

#[test]
#[ignore = "requires private native261 profile/traits and player actor"]
fn subtlety_clone_funding_earned_finisher_clone_can_store_and_energize() {
    let _seed = crate::random::seed_scope(70448);
    let mut funded = false;
    for _ in 0..10 {
        let mut run = prepare_literal(261, 1).unwrap();
        engage(&mut run);
        run.app
            .world_mut()
            .entity_mut(run.actor)
            .remove::<crate::combat::AutoAttack>();
        for id in [196911, 1269163] {
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
        assert!(crate::auras::set_self_cast_stacks(
            run.app.world_mut(),
            run.actor,
            196911,
            7
        ));
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
                196819,
                run.targets[0],
                Some(run.targets[0]),
            ),
        )
        .unwrap();
        assert!(
            !crate::auras::has_aura(run.app.world(), run.actor, 196911),
            "finisher collected all preloaded storage"
        );
        // Keep the real target but move both white swings beyond the clone window;
        // queued AttackStart on the same target preserves these timers.
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
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        assert!(events.iter().any(|event| event.spell_id == Some(1269205) && event.kind == CombatLogKind::Damage), "guaranteed clone must land");
        assert!(
            !events
                .iter()
                .any(|event| event.source == Some(run.actor.to_bits())
                    && event.spell_id.is_none()
                    && event.kind == CombatLogKind::Damage),
            "white swings must not fund this test"
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
        "seeded eligible clone must earn native storage and Energy"
    );
}
