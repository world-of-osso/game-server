use super::follower_dps_diagnostic::*;
use super::*;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with Arms follower profile"]
fn follower_arms_execute_applies_one_deep_wounds_deposit() {
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2515).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 71, 70);
    for _ in 0..5 {
        app.update();
    }
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
    app.init_resource::<crate::spell_cast::test_harness::ObservedTriggeredCasts>();
    crate::spell_cast::creature_cast(app.world_mut(), actor, 281000, Some(target), false).unwrap();
    for _ in 0..20 {
        app.update();
    }
    let casts = &app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedTriggeredCasts>()
        .0;
    let deposits = casts
        .iter()
        .filter(|(caster, spell, _)| *caster == actor && *spell == 262115)
        .count();
    assert_eq!(
        deposits, 1,
        "one Execute impact must deposit Deep Wounds once: {casts:?}"
    );
}
