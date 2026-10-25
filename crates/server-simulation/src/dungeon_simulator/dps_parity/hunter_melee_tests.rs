use super::*;

#[test]
#[ignore = "requires private spec253 reference inputs and native world DB"]
fn native_bm_pet_white_swings_read_live_owner_critical_chance() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(253, 1).expect("native literal BM fixture");
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor).unwrap();
    engage(&mut run);
    crate::pet_control::order_companion_pet(run.app.world_mut(), run.actor, Some(run.targets[0]));
    // The first scheduled stat fold occurs at frame2; settle it before the control.
    for _ in 0..3 {
        run.app.update();
    }
    run.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .crit_pct = 100.0;
    let start = run.app.world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>().0.len();
    for _ in 0..200 {
        run.app.update();
        let live = run.app.world().get::<crate::stats::CharacterStats>(run.actor).unwrap().crit_pct;
        assert_eq!(live, 100.0, "native owner100% control must remain active during the observation window");
    }
    let events = &run.app.world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>().0[start..];
    let swings = events.iter().filter(|event| {
        event.source == Some(pet.to_bits()) && event.spell_id.is_none()
            && event.kind == shared::protocol::CombatLogKind::Damage
    }).collect::<Vec<_>>();
    assert!(!swings.is_empty(), "the main pet must make real white swings");
    let critical = swings.iter().filter(|event| event.crit).count();
    let noncritical = swings.len() - critical;
    eprintln!("NATIVE_PET_OWNER_CRIT live=100 critical={critical} noncritical={noncritical}");
    assert!(critical > 0, "owner's full native crit must reach pet melee");
    assert_eq!(noncritical, 0,
        "pet white swings must inherit the current owner's critical chance, not the creature's fixed crit");
}
