//! A Ferocity follower pet supplies Primal Rage to a party with no other lust.
use super::follower_dps_diagnostic::*;
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn hunter_pet_lusts_once_and_locks_out_the_whole_party() {
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    for spec in [253, 255] {
        let _seed = crate::random::seed_scope(70448);
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
            [66, 105, 71, spec, 103],
        )
        .unwrap();
        let hunter = party.members[3];
        for _ in 0..5 {
            app.update();
        }
        super::party_lust_follower_tests::start_large_trash_pull(&mut app, party.owner, hunter);
        for _ in 0..300 {
            app.update();
        }
        let pet =
            crate::hunter_pet::summoned_pet(app.world(), hunter).expect("hunter summoned its pet");
        let stable = app
            .world()
            .get::<crate::hunter_pet::PetStable>(hunter)
            .unwrap();
        let record = stable.stored.active[0].as_ref().unwrap();
        assert_eq!(record.family, 7, "INFERRED_WCL Ferocity Carrion Bird");
        assert!(crate::hunter_pet::pet_has_specialization_spell(
            app.world(),
            pet,
            264667
        ));
        let casts = app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.kind == CombatLogKind::CastSuccess
                    && event.source == Some(pet.to_bits())
                    && event.spell_id == Some(264667)
            })
            .count();
        assert_eq!(casts, 1, "spec {spec} pet casts Primal Rage once");
        for member in std::iter::once(party.owner).chain(party.members.iter().copied()) {
            assert!(
                crate::auras::bloodlust::has_lockout(app.world(), member),
                "spec {spec} missing lockout on {member:?}"
            );
            assert!(
                crate::auras::has_aura(app.world(), member, 264667),
                "spec {spec} missing Primal Rage on {member:?}"
            );
        }
    }
}
