//! The selected Feral follower earns Ravage from landed white swings.
use super::follower_dps_diagnostic::*;
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn feral_follower_white_swings_earn_ravage() {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 103, 90);
    assert!(
        crate::auras::has_aura(app.world(), actor, 441583),
        "Ravage selected"
    );
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
    let swings = events
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Damage
                && event.spell_id.is_none()
                && event.source == Some(actor.to_bits())
        })
        .count();
    let grants = events
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::AuraApplied
                && event.spell_id == Some(441585)
                && event.source == Some(actor.to_bits())
        })
        .count();
    assert!(
        swings >= 30,
        "the fixed fight must include landed white swings: {swings}"
    );
    assert!(
        grants >= 1 && grants * 3 < swings,
        "selected Ravage is a rare earned proc: {grants} grants from {swings} swings"
    );
}
