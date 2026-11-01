use super::*;
use super::follower_dps_diagnostic::*;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with Arms follower profile"]
fn follower_arms_fatality_mortal_strike_earns_mark_and_execute_deals_damage() {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 71, 90);
    for _ in 0..5 {
        app.update();
    }
    assert!(crate::auras::has_aura(app.world(), actor, 383703));
    let mut successes = 0;
    for _ in 0..16 {
        assert!(crate::spell_cast::try_cast_triggered(
            app.world_mut(),
            actor,
            12294,
            Some(target),
            1,
        ));
        successes += 1;
        app.update();
        if crate::auras::has_aura(app.world(), target, 383704) {
            break;
        }
    }
    assert!(
        app.world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(12294)
                && event.amount > 0),
        "actual Mortal Strike impacts must land"
    );
    assert!(
        crate::auras::has_aura(app.world(), target, 383704),
        "selected Fatality must earn a mark from actual Mortal Strikes; casts={successes}"
    );
    let mut health = app
        .world_mut()
        .get_mut::<shared::components::Health>(target)
        .unwrap();
    health.current = health.max * 0.1;
    let mut cast = false;
    for _ in 0..100 {
        app.world_mut()
            .get_mut::<crate::spell_cast::powers::PowerPools>(actor)
            .unwrap()
            .get_mut(shared::components::PowerType::Rage)
            .unwrap()
            .current = 1000.0;
        if crate::spell_cast::creature_cast(app.world_mut(), actor, 281000, Some(target), false)
            .is_ok()
        {
            cast = true;
            break;
        }
        app.update();
    }
    assert!(cast, "actual Execute must be admitted");
    for _ in 0..20 {
        app.update();
    }
    let events = &app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(383706)
                && event.amount > 0)
    );
}
