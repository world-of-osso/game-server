use super::*;
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires private native260 profile/traits and player actor"]
fn outlaw_blade_flurry_deft_maneuvers_awards_points_per_hit() {
    for talent in [false, true] {
        let _seed = crate::random::seed_scope(70448);
        let mut run = prepare_literal(260, 3).unwrap();
        engage(&mut run);
        let world = run.app.world_mut();
        if !talent {
            let known = world
                .get::<crate::class_progression::KnownSpells>(run.actor)
                .unwrap()
                .iter()
                .filter(|&id| id != 381878)
                .collect();
            world
                .entity_mut(run.actor)
                .insert(crate::class_progression::KnownSpells::fixture(known));
            crate::auras::remove_auras(
                world,
                run.actor,
                crate::auras::RemoveReason::Cancelled,
                |a| a.spell_id == 381878,
            );
        }
        assert_eq!(crate::auras::has_aura(world, run.actor, 381878), talent);
        world
            .get_mut::<crate::spell_cast::powers::PowerPools>(run.actor)
            .unwrap()
            .get_mut(PowerType::ComboPoints)
            .unwrap()
            .current = 0.0;
        literal_rotation::execution::execute(
            world,
            run.actor,
            literal_rotation::execution::Execution::Spell(13877, run.actor, None),
        )
        .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
        for _ in 0..10 {
            run.app.update();
        }
        let points = run
            .app
            .world()
            .get::<crate::spell_cast::powers::PowerPools>(run.actor)
            .unwrap()
            .get(PowerType::ComboPoints)
            .unwrap()
            .current;
        assert_eq!(points, if talent { 3.0 } else { 0.0 });
    }
}

#[test]
#[ignore = "requires private native260 profile/traits and player actor"]
fn outlaw_blade_flurry_activation_hits_each_native_enemy_once() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(260, 3).unwrap();
    engage(&mut run);
    run.app
        .world_mut()
        .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .clear();
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(13877, run.actor, None),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    for _ in 0..10 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    // SimC76f8118 sc_rogue.cpp3974–3999 executes only the sourced
    // activation attack, not the legacy alternate331850 hit as well.
    assert!(
        !events.iter().any(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(331850)
                && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_))
        }),
        "one activation must not also emit the obsolete331850 attack"
    );
    for target in &run.targets {
        let hits: Vec<_> = events
            .iter()
            .filter(|event| {
                event.kind == CombatLogKind::Damage
                    && event.spell_id == Some(429951)
                    && event.source == Some(run.actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.amount > 0
            })
            .collect();
        // SimC76f8118 sc_rogue.cpp3993–3999: activating Blade Flurry
        // executes one independent native area attack, including the primary.
        assert_eq!(
            hits.len(),
            1,
            "native Blade Flurry activation must hit {target:?} once"
        );
    }
}
