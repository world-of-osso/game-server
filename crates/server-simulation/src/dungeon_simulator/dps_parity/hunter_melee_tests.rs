use super::*;

#[test]
#[ignore = "requires private spec253 reference inputs and native world DB"]
fn native_bm_pet_white_swings_read_live_owner_critical_chance() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(253, 1).expect("native literal BM fixture");
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor).unwrap();
    engage(&mut run);
    crate::pet_control::order_companion_pet(run.app.world_mut(), run.actor, Some(run.targets[0]));
    let start = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    let mut minimum = f32::MAX;
    let mut maximum = f32::MIN;
    for _ in 0..200 {
        run.app.update();
        let live = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(run.actor)
            .unwrap()
            .crit_pct;
        assert!(
            live > 0.0 && live < 100.0,
            "native reference crit must allow both outcomes"
        );
        // Derived stats are recomputed when native aura modifiers change. Read
        // that live owner value instead of overwriting its derived cache.
        assert_eq!(
            crate::combat::read_pet_owner_critical_chance(run.app.world(), pet),
            Some(live),
            "pet melee must read the current native owner chance after every stat fold"
        );
        minimum = minimum.min(live);
        maximum = maximum.max(live);
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[start..];
    let swings: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(pet.to_bits())
                && event.spell_id.is_none()
                && event.kind == shared::protocol::CombatLogKind::Damage
        })
        .collect();
    let critical = swings.iter().filter(|event| event.crit).count();
    let noncritical = swings.len() - critical;
    eprintln!(
        "NATIVE_PET_OWNER_CRIT native_range={minimum}..{maximum} critical={critical} noncritical={noncritical}"
    );
    assert!(
        critical > 0 && noncritical > 0,
        "real native pet swings must exercise both outcomes at the reference owner's chance"
    );
}
