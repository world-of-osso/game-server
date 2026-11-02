//! Behavioral tests with the actual level-70 follower.
use super::follower_dps_diagnostic::*;
use super::*;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_demonology_core_demonbolt_reduces_doomguard_cooldown() {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2515).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 266, 70);
    app.world_mut()
        .entity_mut(actor)
        .remove::<crate::followers::Follower>();
    crate::spell_cast::creature_cast(app.world_mut(), actor, 1276672, Some(target), false).unwrap();
    for _ in 0..50 {
        app.update();
    }
    let cooldown = |app: &App| {
        let spell = app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(1276672)
            .unwrap();
        app.world()
            .get::<crate::spell_cast::SpellHistory>(actor)
            .unwrap()
            .remaining_cooldown_ms(spell, crate::spell_cast::history_now(app.world()))
            as f64
            / 1000.0
    };
    crate::spell_cast::cast_triggered(app.world_mut(), actor, 264173, Some(actor), 1);
    assert!(crate::auras::has_aura(app.world(), actor, 264173));
    let before = cooldown(&app);
    let start = crate::spell_cast::history_now(app.world());
    crate::spell_cast::creature_cast(app.world_mut(), actor, 264178, Some(target), false).unwrap();
    for _ in 0..20 {
        app.update();
    }
    let elapsed = crate::spell_cast::history_now(app.world()) - start;
    let reduction = before - elapsed - cooldown(&app);
    assert!(
        (reduction - 3.0).abs() < 0.1,
        "Core Demonbolt must reduce the real running cooldown by 3s: {reduction}"
    );
    assert!(!crate::auras::has_aura(app.world(), actor, 264173));
    let before = cooldown(&app);
    let start = crate::spell_cast::history_now(app.world());
    crate::spell_cast::creature_cast(app.world_mut(), actor, 264178, Some(target), false).unwrap();
    for _ in 0..120 {
        app.update();
    }
    let elapsed = crate::spell_cast::history_now(app.world()) - start;
    let reduction = before - elapsed - cooldown(&app);
    assert!(
        reduction.abs() < 0.1,
        "hardcast without Core must not reduce cooldown: {reduction}"
    );
}
