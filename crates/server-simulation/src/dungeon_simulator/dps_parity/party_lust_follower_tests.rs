use super::follower_dps_diagnostic::*;
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn two_lust_followers_cast_once_and_apply_a_party_lockout() {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let party = crate::followers::simulator::spawn_test_party(
        app.world_mut(),
        copy,
        shared::components::Position {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        90,
        [66, 105, 263, 63, 253],
    )
    .unwrap();
    let shaman = party.members[2];
    let mage = party.members[3];
    let target = spawn_level_dummy(&mut app, shaman, 90, 0).unwrap();
    for _ in 0..5 {
        app.update();
    }
    start_fixed_time_fight(&mut app, party.owner, shaman, target, 120.0);
    for _ in 0..200 {
        app.update();
    }
    let events = &app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let casts: Vec<_> = events.iter().filter(|event| {
        event.kind == CombatLogKind::CastSuccess
            && matches!(event.source, Some(source) if source == mage.to_bits() || source == shaman.to_bits())
            && matches!(event.spell_id, Some(2825 | 32182 | 80353 | 390386))
    }).collect();
    assert_eq!(casts.len(), 1, "only one follower may lust: {casts:?}");
    for member in std::iter::once(party.owner).chain(party.members.iter().copied()) {
        assert!(
            crate::auras::bloodlust::has_lockout(app.world(), member),
            "missing native lockout on {member:?}"
        );
    }
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn mage_and_devastation_followers_use_native_lust_in_fixed_fights() {
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    for (spec, spell, lockout) in [
        (62, 80353, 80354),
        (63, 80353, 80354),
        (64, 80353, 80354),
        (1467, 390386, 390435),
    ] {
        let _seed = crate::random::seed_scope(70448);
        let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
        let copy = create_open_follower_copy(&mut app, 2813).unwrap();
        let (mut app, actor, target, _) = prepare_class_follower(app, copy, spec, 90);
        let owner = app
            .world()
            .get::<crate::followers::Follower>(actor)
            .unwrap()
            .owner;
        for _ in 0..5 {
            app.update();
        }
        start_fixed_time_fight(&mut app, owner, actor, target, 120.0);
        for _ in 0..120 {
            app.update();
        }
        assert!(
            crate::auras::has_aura(app.world(), actor, spell),
            "spec {spec} must gain actual lust"
        );
        assert!(
            crate::auras::has_aura(app.world(), actor, lockout),
            "spec {spec} must gain its native lockout"
        );
        let casts = app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.source == Some(actor.to_bits())
                    && event.kind == CombatLogKind::CastSuccess
                    && event.spell_id == Some(spell)
            })
            .count();
        assert_eq!(casts, 1, "spec {spec} must cast its real lust exactly once");
    }
}
