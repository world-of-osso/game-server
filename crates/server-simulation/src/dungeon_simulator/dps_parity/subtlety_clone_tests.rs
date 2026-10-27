//! SimC76f8118 sc_rogue.cpp8668–8701: a fully funded Ancient Arts
//! finisher guarantees its delayed, combo-point-snapshotted shadow clone.
use super::*;
use shared::{components::PowerType, protocol::CombatLogKind};

#[test]
#[ignore = "requires private native261 profile/traits and player actor"]
fn subtlety_clone_builder_collection_can_launch_native_clone() {
    let _seed = crate::random::seed_scope(70448);
    let mut produced = false;
    for _ in 0..5 {
        let mut run = prepare_literal(261, 1).unwrap();
        engage(&mut run);
        crate::auras::apply_aura(
            run.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: run.actor,
                target: run.actor,
                spell_id: 196911,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
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
            pools.get_mut(PowerType::ComboPoints).unwrap().current = 0.0;
            let energy = pools.get_mut(PowerType::Energy).unwrap();
            energy.current = energy.max;
        }
        literal_rotation::execution::execute(
            run.app.world_mut(),
            run.actor,
            literal_rotation::execution::Execution::Spell(53, run.targets[0], Some(run.targets[0])),
        )
        .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
        produced |= run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.target == Some(run.targets[0].to_bits())
                    && event.spell_id == Some(1269209)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            });
        if produced {
            break;
        }
    }
    assert!(
        produced,
        "seeded actual Backstab storage collection must produce its native shadow clone"
    );
}

#[test]
#[ignore = "requires private native261 profile/traits and player actor"]
fn subtlety_clone_storm_collection_reaches_three_native_recipients() {
    let _seed = crate::random::seed_scope(70448);
    let mut produced = false;
    for _ in 0..10 {
        let mut run = prepare_literal(261, 3).unwrap();
        engage(&mut run);
        crate::auras::apply_aura(
            run.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: run.actor,
                target: run.actor,
                spell_id: 196911,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
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
            pools.get_mut(PowerType::ComboPoints).unwrap().current = 0.0;
            let energy = pools.get_mut(PowerType::Energy).unwrap();
            energy.current = energy.max;
        }
        literal_rotation::execution::execute(
            run.app.world_mut(),
            run.actor,
            literal_rotation::execution::Execution::Spell(
                197835,
                run.targets[0],
                Some(run.targets[0]),
            ),
        )
        .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        produced |= run.targets.iter().all(|target| {
            events.iter().any(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(1269340)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
        });
        if produced {
            break;
        }
    }
    assert!(
        produced,
        "seeded real Storm storage collection must produce its native three-target clone"
    );
}

#[test]
#[ignore = "requires private native261 profile/traits and player actor"]
fn subtlety_clone_eviscerate_snapshots_darkest_night_before_consumption() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(261, 1).unwrap();
    engage(&mut run);
    run.app.world_mut().get_mut::<crate::stats::CharacterStats>(run.actor).unwrap().crit_pct = 0.0;
    for id in [196911, 1269163, 457280] {
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
        literal_rotation::execution::Execution::Spell(196819, run.targets[0], Some(run.targets[0]))).unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let events = &run.app.world().resource::<crate::spell_cast::test_harness::ObservedCombatEvents>().0;
    let clone = events.iter().find(|event| event.source == Some(run.actor.to_bits())
        && event.target == Some(run.targets[0].to_bits()) && event.spell_id == Some(1269205)
        && event.kind == CombatLogKind::Damage && event.amount > 0).expect("earned native Eviscerate clone");
    assert!(clone.crit, "clone must retain Darkest Night's guaranteed critical chance");
}

#[test]
#[ignore = "requires private native261 profile/traits and player actor"]
fn subtlety_clone_ancient_finisher_launches_native_delayed_clone() {
    for (spell, clone, targets) in [(196819, 1269205, 1), (319175, 1269565, 3)] {
        let mut run = prepare_literal(261, targets).unwrap();
        engage(&mut run);
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
                spell,
                run.targets[0],
                Some(run.targets[0]),
            ),
        )
        .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        for target in &run.targets {
            assert!(
                events
                    .iter()
                    .any(|event| event.source == Some(run.actor.to_bits())
                        && event.target == Some(target.to_bits())
                        && event.spell_id == Some(clone)
                        && event.kind == CombatLogKind::Damage
                        && event.amount > 0),
                "the earned finisher must launch its native clone on each recipient"
            );
        }
    }
}
