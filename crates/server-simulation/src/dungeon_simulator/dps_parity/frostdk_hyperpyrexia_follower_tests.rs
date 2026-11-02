//! Two-handed Frost Strike must feed the native Hyperpyrexia residual.
use super::follower_dps_diagnostic::*;
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn frost_two_hand_strikes_produce_hyperpyrexia() {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 251, 90);
    assert!(
        app.world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .off_hand
            .is_none()
    );
    assert!(crate::auras::has_aura(app.world(), actor, 456238));
    // Freeze the follower brain while queued native Frost Strike children resolve.
    assert!(
        app.world_mut()
            .entity_mut(actor)
            .take::<crate::followers::Follower>()
            .is_some()
    );
    for _ in 0..30 {
        crate::spell_cast::cast_triggered(app.world_mut(), actor, 49143, Some(target), 1);
    }
    for _ in 0..200 {
        app.update();
    }
    let events = &app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.spell_id == Some(325464) && event.amount > 0)
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event.spell_id, Some(155166 | 195975))),
        "other Hyperpyrexia sources must not contribute"
    );
    assert!(
        events.iter().any(|event| event.spell_id == Some(458169)
            && event.kind == CombatLogKind::Damage
            && event.periodic
            && event.amount > 0),
        "two-handed Frost Strike must trigger Hyperpyrexia damage"
    );
}
