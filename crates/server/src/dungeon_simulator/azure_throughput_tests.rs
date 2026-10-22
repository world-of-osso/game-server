//! Actual native campaign throughput boundaries, not replacement combat formulas.
use super::super::catalogs::Catalogs;
use super::spawn_run;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "explicit native Azure70 BM leader Call Pet and damage across cached worlds"]
fn native_bm_leader_summons_and_uses_its_pet_without_transport_identity() {
    let _rng_scope = crate::random::seed_scope(1);
    // The frozen cohort predates the duck import. Use the existing authenticated
    // class-summon fixture, not an invented template or a canonical DB write.
    let prepared = crate::spell_cast::test_harness::prepare_class_world();
    let mut catalogs = None;
    for iteration in 0..2 {
        let mut run = spawn_run(1, &mut catalogs).unwrap();
        assert_eq!(run.graph.map_id, 2515);
        let owner = run.party.owner;
        if iteration == 0 {
            let data = prepared.world_data();
            assert!(data.templates.contains_key(&174884));
            run.app.insert_resource(data);
        }
        let mut damaged = false;
        for _ in 0..1200 {
            run.control().unwrap();
            run.app.update();
            if let Some(pet) = crate::hunter_pet::summoned_pet(run.app.world(), owner) {
                damaged = run
                    .app
                    .world()
                    .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                    .0
                    .iter()
                    .any(|event| {
                        event.kind == CombatLogKind::Damage
                            && event.source == Some(pet.to_bits())
                            && event.amount > 0
                    });
                if damaged {
                    break;
                }
            }
        }
        assert!(
            damaged,
            "native BM leader must summon through Call Pet and deal real pet damage in cached world{iteration}; preparation={:?}",
            crate::hunter_pet::provision_follower_duck(run.app.world_mut(), owner)
        );
        let stable = run
            .app
            .world()
            .get::<crate::hunter_pet::PetStable>(owner)
            .unwrap();
        assert_eq!(
            stable.character_id, None,
            "no fabricated persistent identity"
        );
        assert!(
            run.app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .iter()
                .any(|event| event.kind == CombatLogKind::CastSuccess
                    && event.source == Some(owner.to_bits())
                    && event.spell_id == Some(883))
        );
        catalogs = Some(Catalogs::take(run.app.world_mut()));
    }
}
