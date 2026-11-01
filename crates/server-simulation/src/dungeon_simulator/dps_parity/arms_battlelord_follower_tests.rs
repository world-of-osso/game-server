use super::*;
use super::follower_dps_diagnostic::*;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with Arms follower profile"]
fn follower_arms_battlelord_only_overpower_earns_and_mortal_strike_consumes() {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2515).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 71, 70);
    for _ in 0..5 {
        app.update();
    }
    assert!(crate::auras::has_aura(app.world(), actor, 386630));
    for _ in 0..16 {
        assert!(crate::spell_cast::try_cast_triggered(
            app.world_mut(),
            actor,
            1464,
            Some(target),
            1
        ));
        app.update();
    }
    assert!(
        !crate::auras::has_aura(app.world(), actor, 386631),
        "Slam must not earn Battlelord"
    );
    app.world_mut()
        .get_mut::<crate::spell_cast::powers::PowerPools>(actor)
        .unwrap()
        .get_mut(shared::components::PowerType::Rage)
        .unwrap()
        .current = 1000.0;
    crate::spell_cast::creature_cast(app.world_mut(), actor, 12294, Some(target), false).unwrap();
    let on_cooldown = |app: &App| {
        let world = app.world();
        let spell = world.resource::<crate::spell_info::SpellStore>().get(12294).unwrap();
        world.get::<crate::spell_cast::SpellHistory>(actor).unwrap().on_cooldown(spell, crate::spell_cast::history_now(world))
    };
    assert!(on_cooldown(&app), "Mortal Strike starts its native cooldown");
    for _ in 0..16 {
        assert!(crate::spell_cast::try_cast_triggered(
            app.world_mut(),
            actor,
            7384,
            Some(target),
            1
        ));
        app.update();
        if crate::auras::has_aura(app.world(), actor, 386631) {
            break;
        }
    }
    assert!(
        crate::auras::has_aura(app.world(), actor, 386631),
        "actual Overpower casts must earn Battlelord"
    );
    assert!(!on_cooldown(&app), "Battlelord resets the running Mortal Strike cooldown");
    assert!(crate::spell_cast::try_cast_triggered(
        app.world_mut(),
        actor,
        12294,
        Some(target),
        1
    ));
    assert!(
        crate::auras::has_aura(app.world(), actor, 386631),
        "background Mortal Strike must not consume Battlelord"
    );
    for _ in 0..60 {
        app.update();
    }
    app.world_mut()
        .get_mut::<crate::spell_cast::powers::PowerPools>(actor)
        .unwrap()
        .get_mut(shared::components::PowerType::Rage)
        .unwrap()
        .current = 1000.0;
    crate::spell_cast::creature_cast(app.world_mut(), actor, 12294, Some(target), false).unwrap();
    assert!(
        !crate::auras::has_aura(app.world(), actor, 386631),
        "foreground Mortal Strike consumes Battlelord"
    );
}
