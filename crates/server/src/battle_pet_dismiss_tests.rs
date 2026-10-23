//! Real DismissPet RPC follows TC DismissCritter identity, not journal tuple age.
use super::*;

#[test]
fn toy_coverage_critter_dismiss_current_journal_balloon_clears_runtime_not_owned_instance() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, ids) = create_critter_fixture(&dir.path().join("dismiss-balloon.redb"));
    let actor = summon_journal_actor(&mut h, ids.yellow_balloon, YELLOW_BALLOON);
    assert_eq!(
        select_source_targets(&mut h, OWN_CRITTER_SPELL, None),
        Ok(vec![actor])
    );
    let updates = request_dismiss(&mut h);
    assert!(
        updates
            .iter()
            .any(|update| update.error.is_none() && update.summoned_pet_id.is_none()),
        "{updates:?}"
    );
    assert!(h.app.world().get_entity(actor).is_err());
    assert_eq!(
        select_source_targets(&mut h, OWN_CRITTER_SPELL, None),
        Ok(Vec::new())
    );
    let saved = load_journal(&h.app.world().resource::<GameDatabase>().db, 1);
    assert_eq!(
        saved.get(ids.yellow_balloon).unwrap().species_id,
        YELLOW_BALLOON
    );
}

#[test]
fn toy_coverage_critter_dismiss_current_legacy_succeeds_without_active_journal_tuple() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, _) = create_critter_fixture(&dir.path().join("dismiss-legacy.redb"));
    let actors = cast_source_summon(&mut h, COYOTE_SPELL, COYOTE_ENTRY);
    assert_eq!(actors.len(), 1);
    assert_eq!(
        select_source_targets(&mut h, OWN_CRITTER_SPELL, None),
        Ok(actors.clone())
    );
    let updates = request_dismiss(&mut h);
    assert!(
        updates
            .iter()
            .any(|update| update.error.is_none() && update.summoned_pet_id.is_none()),
        "{updates:?}"
    );
    assert!(h.app.world().get_entity(actors[0]).is_err());
    assert_eq!(
        select_source_targets(&mut h, OWN_CRITTER_SPELL, None),
        Ok(Vec::new())
    );
}

#[test]
fn toy_coverage_critter_dismiss_does_not_substitute_guardians_or_noncompanion_journal_actor() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, ids) = create_critter_fixture(&dir.path().join("dismiss-no-substitute.redb"));
    let guardians = cast_source_summon(&mut h, SKELETON_SPELL, SKELETON_ENTRY);
    assert!((2..=3).contains(&guardians.len()));
    let refused = request_dismiss(&mut h);
    assert!(
        refused.iter().any(|update| update.error.is_some()),
        "{refused:?}"
    );
    for guardian in &guardians {
        assert!(h.app.world().get_entity(*guardian).is_ok());
    }
    let ordinary = summon_journal_actor(&mut h, ids.ordinary_ally, HIDDEN_GREEN_BALLOON);
    assert_eq!(
        select_source_targets(&mut h, OWN_CRITTER_SPELL, None),
        Ok(Vec::new())
    );
    let refused = request_dismiss(&mut h);
    assert!(
        refused
            .iter()
            .any(|update| update.error.is_some()
                && update.summoned_pet_id == Some(ids.ordinary_ally)),
        "{refused:?}"
    );
    assert!(h.app.world().get_entity(ordinary).is_ok());
    for guardian in guardians {
        assert!(h.app.world().get_entity(guardian).is_ok());
    }
}

#[test]
fn toy_coverage_critter_dismiss_ignores_selected_foreign_same_species_actor() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, ids) = create_critter_fixture(&dir.path().join("dismiss-owner.redb"));
    let own = summon_journal_actor(&mut h, ids.mechanical, MECHANICAL_SQUIRREL);
    let foreign = spawn_player(&mut h.app, 8, 5.0);
    let connection = spawn_peer_connection(&mut h.app, foreign, 2);
    h.app
        .world_mut()
        .entity_mut(connection)
        .insert((Authenticated { account_id: 2 }, InWorld { character_id: 2 }));
    advance_journal(&mut h, 5);
    take_journal_replies(&mut h, connection);
    let updates = request_summon(&mut h, connection, ids.foreign);
    assert!(
        updates
            .iter()
            .any(|update| update.error.is_none() && update.summoned_pet_id == Some(ids.foreign)),
        "{updates:?}"
    );
    let entry = h
        .app
        .world()
        .resource::<PetCatalog>()
        .creature_id(MECHANICAL_SQUIRREL)
        .unwrap();
    let other = owned_actors(&mut h, foreign, entry)[0];
    h.select(Some(other));
    advance_journal(&mut h, 2);
    let updates = request_dismiss(&mut h);
    assert!(
        updates
            .iter()
            .any(|update| update.error.is_none() && update.summoned_pet_id.is_none()),
        "{updates:?}"
    );
    assert!(h.app.world().get_entity(own).is_err());
    assert!(h.app.world().get_entity(other).is_ok());
    assert_eq!(
        crate::spell_cast::summon::current_critter(h.app.world(), foreign),
        Some(other)
    );
    let again = request_dismiss(&mut h);
    assert!(
        again.iter().any(|update| update.error.is_some()),
        "{again:?}"
    );
    assert!(h.app.world().get_entity(other).is_ok());
    let saved = load_journal(&h.app.world().resource::<GameDatabase>().db, 2);
    assert_eq!(
        saved.get(ids.foreign).unwrap().species_id,
        MECHANICAL_SQUIRREL
    );
}
