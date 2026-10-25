//! Native Feral mechanics, independent of the literal Feral controller.
use super::*;

#[test]
#[ignore = "requires exact spec103 profile/traits and native private DB"]
fn feral_prev_gcd_tracks_successful_foreground_gcd_casts_only() {
    use server::simc_apl::Facts;
    let mut run = prepare_literal(103, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 768, Some(actor), false).unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let bindings = crate::simc_apl_native::Bindings {
        actions: [("shred".into(), 5221), ("rake".into(), 1822)].into(),
        ..Default::default()
    };
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let read = |run: &Run, name: &str| {
        crate::simc_apl_native::NativeFacts::new(run.app.world(), actor, &bindings, encounter, 1)
            .with_targets(Some(target), &run.targets)
            .value(name)
            .unwrap()
    };
    assert_eq!(read(&run, "prev_gcd.1.shred"), 0.0);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5221, Some(target), false)
        .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    assert_eq!(read(&run, "prev_gcd.1.shred"), 1.0);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5217, Some(actor), false).unwrap();
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1822, Some(target), 1);
    assert!(
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5221, Some(actor), false)
            .is_err()
    );
    assert_eq!(
        read(&run, "prev_gcd.1.shred"),
        1.0,
        "off-GCD, triggered and failed requests preserve history"
    );
    preparation::finish_pending_setup(&mut run).unwrap();
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 1822, Some(target), false)
        .unwrap();
    assert_eq!(read(&run, "prev_gcd.1.rake"), 1.0);
    assert_eq!(read(&run, "prev_gcd.2.shred"), 1.0);
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec103 exact profile/traits"]
fn feral_frenzy_selected_profile_executes_native_children_and_earns_combo_points() {
    let mut run = prepare_literal(103, 1).expect("selected Feral profile");
    let actor = run.actor;
    let target = run.targets[0];
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 768, Some(actor), false)
        .expect("ordinary native Cat Form");
    preparation::finish_pending_setup(&mut run).unwrap();
    let action = crate::spell_cast::knowledge::resolve_actor_action(run.app.world(), actor, 274837)
        .expect("selected Frenzy action");
    run.app
        .world_mut()
        .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .clear();
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, action, Some(target), false)
        .expect("ordinary Frenzy cast");
    for _ in 0..100 {
        run.app.update();
    }
    let combat = report::combat(run.app.world(), &run.targets, 5.0);
    let child = if action == 1243807 { 1244079 } else { 274838 };
    let damage: u64 = combat["actions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["spell_id"] == child)
        .map(|row| row["damage"].as_u64().unwrap())
        .sum();
    assert!(
        damage > 0,
        "native Frenzy must execute its damaging child, not only the dummy parent"
    );
    let combo = crate::spell_cast::powers::power_value(
        run.app.world(),
        actor,
        shared::components::PowerType::ComboPoints,
    )
    .unwrap()
    .0;
    assert_eq!(combo, 5.0, "Frenzy earns the native five combo points");
}
