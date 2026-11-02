//! Blade Rush must deal damage and restore energy in a real follower fight.
use super::follower_dps_diagnostic::*;
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn follower_blade_rush_deals_damage_and_restores_energy() {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2515).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 260, 70);
    let owner = app
        .world()
        .get::<crate::followers::Follower>(actor)
        .unwrap()
        .owner;
    for _ in 0..5 {
        app.update();
    }
    start_fixed_time_fight(&mut app, owner, actor, target, 120.0);
    for frame in 0..FIXED_FIGHT_FRAMES {
        let mut health = app
            .world_mut()
            .get_mut::<shared::components::Health>(target)
            .unwrap();
        health.current =
            health.max * (FIXED_FIGHT_FRAMES - frame) as f32 / FIXED_FIGHT_FRAMES as f32;
        app.update();
    }
    let events = &app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events.iter().any(|e| e.source == Some(actor.to_bits())
            && e.target == Some(target.to_bits())
            && e.kind == CombatLogKind::Damage
            && e.spell_id == Some(271881)
            && e.amount > 0),
        "Blade Rush attack must hit the target"
    );
    assert!(
        events.iter().any(|e| e.source == Some(actor.to_bits())
            && e.target == Some(actor.to_bits())
            && e.kind == CombatLogKind::Energize
            && e.spell_id == Some(271896)
            && e.amount > 0),
        "Blade Rush must restore energy"
    );
}
