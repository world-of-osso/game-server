use super::follower_dps_diagnostic::*;
use super::*;
use shared::protocol::CombatLogKind;

/// The follower policy lusts only on production pulls (a fixed fight takes its lust
/// from the SimC reference): engage `lust_policy::TRASH_ENGAGED` (8) non-boss dummies
/// in front of `anchor`, a large trash pull, which lusts after its 7.7 s delay.
pub(super) fn start_large_trash_pull(app: &mut App, owner: Entity, anchor: Entity) {
    let pack: Vec<Entity> = (0..8)
        .map(|index| spawn_level_dummy(app, anchor, 90, index).unwrap())
        .collect();
    let world = app.world_mut();
    world.entity_mut(owner).insert((
        crate::networking::CombatTarget(Some(pack[0])),
        crate::combat::AutoAttack::new(pack[0]),
    ));
    for target in pack {
        crate::combat_refs::set_in_combat_with(world, owner, target);
    }
}

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
        [66, 105, 263, 63, 103],
    )
    .unwrap();
    let shaman = party.members[2];
    let mage = party.members[3];
    for _ in 0..5 {
        app.update();
    }
    start_large_trash_pull(&mut app, party.owner, shaman);
    for _ in 0..300 {
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
fn mage_and_devastation_followers_use_native_lust_on_a_large_pull() {
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
        let (mut app, actor, _, _) = prepare_class_follower(app, copy, spec, 90);
        let owner = app
            .world()
            .get::<crate::followers::Follower>(actor)
            .unwrap()
            .owner;
        for _ in 0..5 {
            app.update();
        }
        start_large_trash_pull(&mut app, owner, actor);
        for _ in 0..300 {
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
