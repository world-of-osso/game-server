//! Feral attacks extend all existing owned Dreadful Wounds up to the native cap.
use super::follower_dps_diagnostic::*;
use super::*;
use shared::components::PowerType;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn feral_follower_attacks_extend_owned_wounds_to_cap() {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let (mut app, actor, primary, _) = prepare_class_follower(app, copy, 103, 90);
    let secondary = spawn_level_dummy(&mut app, actor, 90, 1).unwrap();
    assert!(
        crate::auras::has_aura(app.world(), actor, 441829),
        "Aggravate Wounds selected"
    );
    crate::spell_cast::cast_triggered(app.world_mut(), actor, 768, Some(actor), 1);
    for target in [primary, secondary] {
        crate::spell_cast::cast_triggered(app.world_mut(), actor, 441812, Some(target), 1);
    }
    let expiry = |app: &bevy::prelude::App, target: Entity| {
        app.world()
            .get::<crate::auras::ActiveAuras>(target)
            .unwrap()
            .iter()
            .find(|a| a.spell_id == 441812 && a.caster == actor)
            .unwrap()
            .expires_at
            .unwrap()
    };
    let initial = [expiry(&app, primary), expiry(&app, secondary)];
    crate::spell_cast::cast_triggered(app.world_mut(), actor, 5217, Some(actor), 1);
    assert_eq!(
        [expiry(&app, primary), expiry(&app, secondary)],
        initial,
        "unrelated Tiger's Fury does not extend wounds"
    );
    for _ in 0..5 {
        app.world_mut()
            .entity_mut(actor)
            .remove::<crate::spell_cast::SpellHistory>();
        crate::spell_cast::powers::change_power(app.world_mut(), actor, PowerType::Energy, 100.0);
        crate::spell_cast::creature_cast(app.world_mut(), actor, 5221, Some(primary), false)
            .unwrap();
    }
    for (index, target) in [primary, secondary].into_iter().enumerate() {
        assert!(
            (expiry(&app, target) - initial[index] - 2.0).abs() < 0.001,
            "five Shreds extend each six-second wound by only two seconds to the eight-second cap"
        );
    }
}
