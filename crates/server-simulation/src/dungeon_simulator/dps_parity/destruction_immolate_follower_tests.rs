//! A below-hero Destruction follower maintains its learned Immolate.
use super::follower_dps_diagnostic::*;
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_destruction_without_wither_maintains_immolate() {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2515).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 267, 70);
    let known = app
        .world()
        .get::<crate::class_progression::KnownSpells>(actor)
        .unwrap();
    assert!(known.contains(348));
    assert!(!known.contains(445465), "L70 does not learn Wither");
    let owner = app
        .world()
        .get::<crate::followers::Follower>(actor)
        .unwrap()
        .owner;
    for _ in 0..5 {
        app.update();
    }
    start_fixed_time_fight(&mut app, owner, actor, target, 120.0);
    for _ in 0..1600 {
        app.update();
    }
    let events = &app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let casts = events
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.spell_id == Some(348)
                && event.kind == CombatLogKind::CastSuccess
        })
        .count();
    let periodic = events
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(157736)
                && event.periodic
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count();
    assert!(casts >= 2, "follower must refresh Immolate: {casts} casts");
    assert!(periodic > 0, "Immolate must deal periodic damage");
}
