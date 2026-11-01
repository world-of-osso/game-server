//! Behavioral tests with the actual level-70 follower.
use super::follower_dps_diagnostic::*;
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_demonology_core_doom_expires_from_hand_of_guldan() {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2515).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 266, 70);
    app.world_mut()
        .entity_mut(actor)
        .remove::<crate::followers::Follower>();
    crate::auras::remove_auras(
        app.world_mut(),
        actor,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 264173,
    );
    assert!(!crate::auras::has_aura(app.world(), actor, 264173));
    crate::spell_cast::creature_cast(app.world_mut(), actor, 264178, Some(target), false).unwrap();
    for frame in 0..120 {
        app.update();
        if crate::auras::has_aura(app.world(), actor, 264173)
            || crate::auras::has_aura(app.world(), target, 460553)
        {
            println!(
                "frame={frame} core={} doom={}",
                crate::auras::has_aura(app.world(), actor, 264173),
                crate::auras::has_aura(app.world(), target, 460553)
            );
        }
    }
    assert!(
        !crate::auras::has_aura(app.world(), target, 460553),
        "hardcast without Core must not apply Doom"
    );
    crate::spell_cast::cast_triggered(app.world_mut(), actor, 264173, Some(actor), 1);
    crate::spell_cast::creature_cast(app.world_mut(), actor, 264178, Some(target), false).unwrap();
    for _ in 0..40 {
        app.update();
    }
    let expiry = |app: &App| {
        app.world()
            .get::<crate::auras::ActiveAuras>(target)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 460553 && aura.caster == actor)
            .and_then(|aura| aura.expires_at)
    };
    let first = expiry(&app).expect("actual Core Demonbolt earns native Doom");
    crate::spell_cast::cast_triggered(app.world_mut(), actor, 264173, Some(actor), 1);
    crate::spell_cast::creature_cast(app.world_mut(), actor, 264178, Some(target), false).unwrap();
    for _ in 0..40 {
        app.update();
    }
    assert_eq!(
        expiry(&app),
        Some(first),
        "another Demonbolt must not refresh active Doom"
    );
    crate::spell_cast::powers::change_power(
        app.world_mut(),
        actor,
        shared::components::PowerType::SoulShards,
        100.0,
    );
    crate::spell_cast::creature_cast(app.world_mut(), actor, 105174, Some(target), false).unwrap();
    for _ in 0..40 {
        app.update();
    }
    let shortened = expiry(&app).expect("Doom remains active");
    assert!(
        (first - shortened - 3.0).abs() < 0.05,
        "actual Hand of Gul'dan removes three seconds"
    );
    assert!(crate::auras::set_owned_duration(
        app.world_mut(),
        actor,
        target,
        460553,
        2000
    ));
    crate::spell_cast::powers::change_power(
        app.world_mut(),
        actor,
        shared::components::PowerType::SoulShards,
        100.0,
    );
    crate::spell_cast::creature_cast(app.world_mut(), actor, 105174, Some(target), false).unwrap();
    for _ in 0..40 {
        app.update();
    }
    assert!(
        expiry(&app).is_none(),
        "Hand of Gul'dan expires Doom when reduction reaches zero"
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
                && event.spell_id == Some(460555)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0),
        "expired Doom must deal native damage"
    );
}
