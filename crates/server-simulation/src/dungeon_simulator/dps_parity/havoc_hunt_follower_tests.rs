//! Exercise The Hunt through a production Havoc follower's rotation.
use super::follower_dps_diagnostic::*;
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with Havoc follower data"]
fn follower_havoc_casts_hunt_and_applies_its_native_damage_and_dot() {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2515).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 577, 70);
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
    let has_event = |spell, kind| {
        events.iter().any(|event| {
            event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(spell)
                && event.kind == kind
        })
    };
    assert!(
        has_event(370965, CombatLogKind::CastSuccess),
        "follower must cast The Hunt"
    );
    assert!(
        has_event(370966, CombatLogKind::Damage),
        "Hunt must deal its native impact damage"
    );
    assert!(
        has_event(370969, CombatLogKind::AuraApplied),
        "Hunt must apply its native DoT"
    );
}
