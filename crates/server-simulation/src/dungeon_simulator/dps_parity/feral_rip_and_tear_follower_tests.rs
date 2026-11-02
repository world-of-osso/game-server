//! Rip and Tear derives its actual residual from the follower's earned Rip.
use super::follower_dps_diagnostic::*;
use super::*;
use shared::{components::PowerType, protocol::CombatLogKind};

fn apply_rip(points: f32) -> (f32, bool) {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 103, 90);
    assert!(
        crate::auras::has_aura(app.world(), actor, 391347),
        "Rip and Tear selected"
    );
    crate::spell_cast::cast_triggered(app.world_mut(), actor, 768, Some(actor), 1);
    crate::spell_cast::powers::change_power(app.world_mut(), actor, PowerType::ComboPoints, points);
    crate::spell_cast::powers::change_power(app.world_mut(), actor, PowerType::Energy, 100.0);
    crate::spell_cast::creature_cast(app.world_mut(), actor, 1079, Some(target), false).unwrap();
    let amount = app
        .world()
        .get::<crate::auras::ActiveAuras>(target)
        .unwrap()
        .iter()
        .find(|a| a.spell_id == 391356 && a.caster == actor)
        .expect("landed Rip applies Tear")
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount;
    for _ in 0..50 {
        app.update();
    }
    let damage = app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .any(|event| {
            event.kind == CombatLogKind::Damage
                && event.spell_id == Some(391356)
                && event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.amount > 0
        });
    (amount, damage)
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn feral_follower_rip_earns_scaled_tear_and_deals_ticks() {
    let one = apply_rip(1.0);
    let five = apply_rip(5.0);
    assert!(
        one.0 > 0.0 && five.0 > one.0 * 2.0,
        "longer paid Rip must contribute more Tear: {one:?}, {five:?}"
    );
    assert!(
        one.1 && five.1,
        "both residuals deal actual periodic damage"
    );
}
