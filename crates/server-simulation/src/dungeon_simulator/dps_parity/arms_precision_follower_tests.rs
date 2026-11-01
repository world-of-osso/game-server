use super::follower_dps_diagnostic::*;
use super::*;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with Arms follower profile"]
fn follower_arms_mortal_strike_consumes_earned_executioners_precision() {
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2515).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 71, 70);
    for _ in 0..5 {
        app.update();
    }
    assert!(crate::auras::has_aura(app.world(), actor, 386634));
    let mut health = app
        .world_mut()
        .get_mut::<shared::components::Health>(target)
        .unwrap();
    health.current = health.max * 0.1;
    app.world_mut()
        .get_mut::<crate::spell_cast::powers::PowerPools>(actor)
        .unwrap()
        .get_mut(shared::components::PowerType::Rage)
        .unwrap()
        .current = 400.0;
    crate::spell_cast::creature_cast(app.world_mut(), actor, 281000, Some(target), false).unwrap();
    for _ in 0..20 {
        app.update();
    }
    assert!(
        crate::auras::has_aura(app.world(), actor, 386633),
        "Execute must earn native Precision"
    );
    assert!(crate::spell_cast::try_cast_triggered(
        app.world_mut(),
        actor,
        7384,
        Some(target),
        1
    ));
    assert!(
        crate::auras::has_aura(app.world(), actor, 386633),
        "Overpower must not consume Precision"
    );
    assert!(crate::spell_cast::try_cast_triggered(
        app.world_mut(),
        actor,
        12294,
        Some(target),
        1
    ));
    assert!(
        !crate::auras::has_aura(app.world(), actor, 386633),
        "Mortal Strike consumes Precision after damage"
    );
    let events = &app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(12294)
                && event.amount > 0)
    );
}
