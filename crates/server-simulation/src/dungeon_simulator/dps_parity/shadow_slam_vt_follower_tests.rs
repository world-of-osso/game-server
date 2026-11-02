use super::follower_dps_diagnostic::*;
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_shadow_slam_applies_periodic_vt_without_direct_damage() {
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2515).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 258, 70);
    app.world_mut()
        .entity_mut(actor)
        .remove::<crate::followers::Follower>();
    crate::spell_cast::creature_cast(app.world_mut(), actor, 1227280, Some(target), false).unwrap();
    for _ in 0..100 {
        app.update();
    }
    assert!(crate::auras::has_aura(app.world(), target, 34914));
    assert!(crate::auras::has_aura(app.world(), target, 589));
    let events = &app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let vt_damage: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.spell_id == Some(34914)
                && event.kind == CombatLogKind::Damage
        })
        .collect();
    assert!(vt_damage.iter().any(|event| event.periodic));
    assert!(
        vt_damage.iter().all(|event| event.periodic),
        "Slam-triggered VT must not deal direct damage: {vt_damage:?}"
    );
}
